use super::client::WebDeckAdminClient;
use super::error::{AdminError, ErrorKind, Result};
use super::output::Outcome;
use super::read_value;
use crate::contracts::Command;
use serde_json::{json, Value};

fn object_from_args(
    action: Option<&str>,
    file: Option<&str>,
    args_json: Option<&str>,
    args: &[(String, String)],
) -> Result<Value> {
    let mut object = match file {
        Some(path) => read_value(Some(path))?
            .as_object()
            .cloned()
            .ok_or_else(|| AdminError::invalid_arguments("Command input must be a JSON object"))?,
        None => {
            let action = action.ok_or_else(|| {
                AdminError::invalid_arguments("action run requires an action type or --file")
            })?;
            let mut object = serde_json::Map::new();
            object.insert("type".into(), json!(action));
            object
        }
    };
    if let Some(action) = action {
        object.insert("type".into(), json!(action));
    }
    if let Some(text) = args_json {
        let value: Value = serde_json::from_str(text)
            .map_err(|_| AdminError::invalid_arguments("--args must be a JSON object"))?;
        let parsed = value
            .as_object()
            .ok_or_else(|| AdminError::invalid_arguments("--args must be a JSON object"))?;
        for (key, value) in parsed {
            object.insert(key.clone(), value.clone());
        }
    }
    for (key, value) in args {
        let parsed = serde_json::from_str(value).unwrap_or_else(|_| json!(value));
        object.insert(key.clone(), parsed);
    }
    Ok(Value::Object(object))
}

pub async fn list(client: &WebDeckAdminClient) -> Result<Outcome> {
    let catalog = client.catalog().await?;
    let actions: Vec<Value> = catalog
        .commands
        .iter()
        .map(|c| json!({"id": c.id, "capability": c.capability}))
        .collect();
    let plugins: Vec<Value> = catalog
        .plugins
        .iter()
        .map(|p| {
            json!({
                "id": p.id,
                "version": p.version,
                "actions": p.actions.iter().map(|a| a.id.clone()).collect::<Vec<_>>()
            })
        })
        .collect();
    let action_count = actions.len();
    let mut rows = catalog
        .commands
        .iter()
        .map(|c| format!("{}\t{}", c.id, c.label.as_deref().unwrap_or(&c.id)))
        .collect::<Vec<_>>();
    rows.extend(catalog.plugins.iter().flat_map(|p| {
        p.actions
            .iter()
            .map(move |a| format!("{}\t{}", plugin_action_id(&p.id, &a.id), a.label))
    }));
    Ok(Outcome::ok(
        json!({
            "api_version": catalog.api_version,
            "actions": actions,
            "plugins": plugins
        }),
        format!(
            "{action_count} action type(s), {} plugin(s).\n{}",
            plugins.len(),
            rows.join("\n")
        ),
    ))
}

pub async fn describe(client: &WebDeckAdminClient, action_id: &str) -> Result<Outcome> {
    let catalog = client.catalog().await?;
    let Some(entry) = catalog.commands.iter().find(|c| c.id == action_id) else {
        let plugin_action = catalog
            .plugins
            .iter()
            .flat_map(|p| p.actions.iter().map(|a| (p.id.clone(), a)))
            .find(|(plugin_id, action)| {
                action.id == action_id || plugin_action_id(plugin_id, &action.id) == action_id
            });
        return match plugin_action {
            Some((plugin_id, action)) => {
                let data = json!({
                    "id": action_id,
                    "plugin": plugin_id,
                    "capability": action.capabilities,
                    "arguments": action.arguments,
                    "result": action.result,
                    "label": action.label
                });
                let pretty = serde_json::to_string_pretty(&data).map_err(|_| {
                    AdminError::new(ErrorKind::Generic, "Cannot encode action schema")
                })?;
                Ok(Outcome::ok(data, pretty))
            }
            None => Err(AdminError::with_details(
                ErrorKind::NotFound,
                format!("Unknown action '{action_id}'"),
                json!({"id": action_id}),
            )),
        };
    };
    let data = json!({
        "id": entry.id,
        "capability": entry.capability,
        "schema": entry.schema,
        "label": entry.label,
        "result_schema": entry.result_schema
    });
    let pretty = serde_json::to_string_pretty(&entry.schema)
        .map_err(|_| AdminError::new(ErrorKind::Generic, "Cannot encode schema"))?;
    Ok(Outcome::ok(
        data,
        format!("Action '{}':\n{pretty}", entry.id),
    ))
}

fn plugin_action_id(plugin_id: &str, action_id: &str) -> String {
    format!("{plugin_id}.{action_id}")
}

pub async fn run(
    client: &WebDeckAdminClient,
    action: Option<&str>,
    file: Option<&str>,
    args_json: Option<&str>,
    args: &[(String, String)],
    dry_run: bool,
) -> Result<Outcome> {
    let catalog = client.catalog().await?;
    let mut value = object_from_args(action, file, args_json, args)?;
    if let Some(id) = value["type"]
        .as_str()
        .filter(|id| !catalog.commands.iter().any(|c| c.id == *id))
    {
        let matches = catalog
            .plugins
            .iter()
            .flat_map(|p| p.actions.iter().map(move |a| (p, a)))
            .filter(|(p, a)| plugin_action_id(&p.id, &a.id) == id || a.id == id)
            .collect::<Vec<_>>();
        if matches.len() > 1 {
            return Err(AdminError::invalid_arguments(
                "Ambiguous plugin action; use plugin_id.action_id",
            ));
        }
        if let Some((plugin, action)) = matches.first() {
            let mut arguments = value.as_object().cloned().unwrap_or_default();
            arguments.remove("type");
            value = json!({"type":"plugin", "plugin_id":plugin.id, "version":plugin.version, "action_id":action.id, "args":arguments});
        }
    }
    let command: Command = serde_json::from_value(value.clone()).map_err(|e| {
        AdminError::with_details(
            ErrorKind::InvalidArguments,
            "Command arguments do not match the expected shape",
            json!({"reason": e.to_string()}),
        )
    })?;
    crate::automation::validate_catalog_command(&command, &catalog)
        .map_err(AdminError::from_domain)?;
    let command_type = value
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();
    if dry_run {
        return Ok(Outcome::ok(
            json!({
                "operation": "action_run",
                "action": command_type,
                "applied": false,
                "dry_run": true,
                "command": value
            }),
            format!("Dry run: would run '{command_type}'."),
        ));
    }
    let result = client.run_command(command.clone()).await?;
    let human = super::output::render_command(&command, &result, Some(&catalog));
    Ok(Outcome::ok(
        json!({
            "operation": "action_run",
            "action": command_type,
            "state": "completed",
            "result": result
        }),
        human,
    ))
}
