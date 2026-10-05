use super::client::{MutationReport, WebDeckAdminClient};
use super::error::{AdminError, ErrorInfo, ErrorKind, ExitCode, Result};
use super::output::Outcome;
use super::{read_value, report_value};
use crate::contracts::{Config, Folder};
use crate::domain;
use serde_json::{json, Value};

pub fn redact_config(config: &Config) -> Result<Value> {
    let mut value = serde_json::to_value(config)
        .map_err(|_| AdminError::new(ErrorKind::Generic, "Cannot encode configuration"))?;
    let settings = value.get_mut("settings").cloned().unwrap_or(Value::Null);
    if let Some(settings) = settings.as_object() {
        let mut settings = settings.clone();
        if let Some(obs) = settings.get_mut("obs").and_then(|o| o.as_object_mut()) {
            if obs
                .get("password")
                .and_then(|p| p.as_str())
                .is_some_and(|p| !p.is_empty())
            {
                obs.insert("password".into(), json!("password_configured"));
            }
        }
        if let Some(spotify) = settings.get_mut("spotify").and_then(|s| s.as_object_mut()) {
            if let Some(secret) = spotify.get("client_secret").and_then(|s| s.as_str()) {
                let configured = !secret.is_empty();
                spotify.insert("client_secret".into(), json!(configured));
            }
        }
        value["settings"] = Value::Object(settings);
    }
    Ok(value)
}

fn encode(value: &Value) -> Result<String> {
    serde_json::to_string_pretty(value)
        .map_err(|_| AdminError::new(ErrorKind::Generic, "Cannot encode output"))
}

fn validation_errors(config: &Config) -> Vec<Value> {
    match domain::validate_config(config) {
        Ok(()) => Vec::new(),
        Err(e) => vec![json!({"code": e.code, "message": e.message})],
    }
}

fn parse_config(value: &Value) -> std::result::Result<Config, Vec<Value>> {
    match serde_json::from_value::<Config>(value.clone()) {
        Ok(config) => {
            let errors = validation_errors(&config);
            if errors.is_empty() {
                Ok(config)
            } else {
                Err(errors)
            }
        }
        Err(e) => Err(vec![
            json!({"code": "invalid_input", "message": format!("Invalid configuration shape: {e}")}),
        ]),
    }
}

fn folder_by_id<'a>(config: &'a Config, id: &str) -> Option<&'a Folder> {
    config.layout.folders.iter().find(|f| f.id == id)
}

fn button_order_changed(current: &Folder, target: &Folder) -> bool {
    let current_order: Vec<&str> = current
        .buttons
        .iter()
        .filter(|b| target.buttons.iter().any(|t| t.id == b.id))
        .map(|b| b.id.as_str())
        .collect();
    let target_order: Vec<&str> = target
        .buttons
        .iter()
        .filter(|b| current.buttons.iter().any(|c| c.id == b.id))
        .map(|b| b.id.as_str())
        .collect();
    current_order != target_order
}

pub fn diff_configs(current: &Config, target: &Config) -> Vec<Value> {
    let mut changes = Vec::new();
    if current.settings != target.settings || current.extensions != target.extensions {
        changes.push(json!({"operation": "update_settings"}));
    }
    if current.layout.columns != target.layout.columns
        || current.layout.rows != target.layout.rows
        || current.layout.themes != target.layout.themes
        || current.layout.backgrounds != target.layout.backgrounds
        || current.layout.extensions != target.layout.extensions
    {
        changes.push(json!({"operation": "update_layout"}));
    }
    for target_folder in &target.layout.folders {
        let Some(current_folder) = folder_by_id(current, &target_folder.id) else {
            changes.push(json!({"operation": "create_folder", "id": target_folder.id}));
            continue;
        };
        if current_folder.label != target_folder.label
            || current_folder.extensions != target_folder.extensions
        {
            changes.push(json!({"operation": "update_folder", "id": target_folder.id}));
        }
        for target_button in &target_folder.buttons {
            match current_folder
                .buttons
                .iter()
                .find(|b| b.id == target_button.id)
            {
                None => changes.push(json!({
                    "operation": "create_button",
                    "id": target_button.id,
                    "folder": target_folder.id
                })),
                Some(current_button) if current_button != target_button => changes.push(json!({
                    "operation": "update_button",
                    "id": target_button.id,
                    "folder": target_folder.id
                })),
                Some(_) => {}
            }
        }
        for current_button in &current_folder.buttons {
            if !target_folder
                .buttons
                .iter()
                .any(|b| b.id == current_button.id)
            {
                changes.push(json!({
                    "operation": "delete_button",
                    "id": current_button.id,
                    "folder": target_folder.id
                }));
            }
        }
        if button_order_changed(current_folder, target_folder) {
            changes.push(json!({"operation": "reorder_buttons", "id": target_folder.id}));
        }
    }
    for current_folder in &current.layout.folders {
        if folder_by_id(target, &current_folder.id).is_none() {
            changes.push(json!({"operation": "delete_folder", "id": current_folder.id}));
        }
    }
    changes
}

fn changes_payload(revision: u64, changes: Vec<Value>) -> Value {
    let change_count = changes.len();
    json!({"revision": revision, "changes": changes, "change_count": change_count})
}

pub async fn get(client: &WebDeckAdminClient, reveal: bool) -> Result<Outcome> {
    let response = client.get_config().await?;
    let config = if reveal {
        serde_json::to_value(&response.config)
            .map_err(|_| AdminError::new(ErrorKind::Generic, "Cannot encode configuration"))?
    } else {
        redact_config(&response.config)?
    };
    let pretty = encode(&config)?;
    let human = format!("revision {}\n{pretty}", response.revision);
    Ok(Outcome::ok(
        json!({
            "revision": response.revision,
            "api_version": response.api_version,
            "config": config,
            "redacted": !reveal
        }),
        human,
    ))
}

pub async fn validate(client: &WebDeckAdminClient, file: Option<&str>) -> Result<Outcome> {
    let (value, source) = match file {
        Some(path) => (read_value(Some(path))?, path.to_string()),
        None => {
            let response = client.get_config().await?;
            let value = serde_json::to_value(&response.config)
                .map_err(|_| AdminError::new(ErrorKind::Generic, "Cannot encode configuration"))?;
            (value, "server".to_string())
        }
    };
    match parse_config(&value) {
        Ok(_) => Ok(Outcome::ok(
            json!({"valid": true, "errors": [], "source": source}),
            format!("Configuration is valid ({source})."),
        )),
        Err(errors) => {
            let mut human = format!("Configuration is invalid ({source}):");
            for error in &errors {
                human.push_str(&format!(
                    "\n  - [{}] {}",
                    error["code"].as_str().unwrap_or("invalid"),
                    error["message"].as_str().unwrap_or("")
                ));
            }
            Ok(Outcome::failed(
                json!({"valid": false, "errors": errors, "source": source}),
                ErrorInfo {
                    code: ErrorKind::Validation.code(),
                    message: "Configuration is invalid".into(),
                    details: Some(json!({"source": source})),
                },
                ExitCode::Validation as i32,
                human,
            ))
        }
    }
}

pub async fn diff(client: &WebDeckAdminClient, file: Option<&str>) -> Result<Outcome> {
    let file = file.ok_or_else(|| {
        AdminError::invalid_arguments("config diff requires --file or stdin with '-'")
    })?;
    let value = read_value(Some(file))?;
    let target = parse_config(&value).map_err(|errors| {
        AdminError::with_details(
            ErrorKind::Validation,
            "Configuration is invalid",
            json!({"errors": errors}),
        )
    })?;
    let current = client.get_config().await?;
    let changes = diff_configs(&current.config, &target);
    let payload = changes_payload(current.revision, changes);
    let change_count = payload["change_count"].as_u64().unwrap_or(0);
    Ok(Outcome::ok(
        payload,
        format!(
            "Revision {}: {change_count} pending change(s).",
            current.revision
        ),
    ))
}

pub async fn apply(
    client: &WebDeckAdminClient,
    file: Option<&str>,
    revision: Option<u64>,
    dry_run: bool,
) -> Result<Outcome> {
    let file = file.ok_or_else(|| {
        AdminError::invalid_arguments("config apply requires --file or stdin with '-'")
    })?;
    let value = read_value(Some(file))?;
    let target = parse_config(&value).map_err(|errors| {
        AdminError::with_details(
            ErrorKind::Validation,
            "Configuration is invalid",
            json!({"errors": errors}),
        )
    })?;
    let current = client.get_config().await?;
    if let Some(expected) = revision {
        client.ensure_revision(Some(expected), current.revision)?;
    }
    let expected = revision.unwrap_or(current.revision);
    let changes = diff_configs(&current.config, &target);
    let change_count = changes.len();
    if change_count == 0 {
        let report = report_value(
            &MutationReport {
                operation: "config_apply".into(),
                revision: current.revision,
                previous_revision: None,
                applied: false,
                dry_run,
            },
            json!({"changes": [], "change_count": 0}),
        );
        return Ok(Outcome::ok(
            report,
            format!(
                "Configuration is already up to date (revision {}).",
                current.revision
            ),
        ));
    }
    if dry_run {
        let report = report_value(
            &MutationReport::planned("config_apply", current.revision),
            json!({"changes": changes, "change_count": change_count}),
        );
        return Ok(Outcome::ok(
            report,
            format!(
                "Dry run: {change_count} change(s) would be applied to revision {}.",
                current.revision
            ),
        ));
    }
    let response = match client.put_config(expected, target).await {
        Ok(response) => response,
        Err(error) => return Err(client.enrich_conflict(error, expected).await),
    };
    let report = report_value(
        &MutationReport::applied("config_apply", expected, response.revision),
        json!({"changes": changes, "change_count": change_count}),
    );
    Ok(Outcome::ok(
        report,
        format!(
            "Applied {change_count} change(s) (revision {expected} -> {}).",
            response.revision
        ),
    ))
}

pub async fn export(client: &WebDeckAdminClient, file: Option<&str>) -> Result<Outcome> {
    let response = client.get_config().await?;
    let mut bytes = serde_json::to_vec_pretty(&response.config)
        .map_err(|_| AdminError::new(ErrorKind::Generic, "Cannot encode configuration"))?;
    bytes.push(b'\n');
    match file {
        Some(path) if path != "-" => {
            super::write_private(std::path::Path::new(path), &bytes)?;
            Ok(Outcome::ok(
                json!({"revision": response.revision, "path": path, "secrets": true}),
                format!("Exported revision {} to {path}.", response.revision),
            ))
        }
        _ => {
            let text = String::from_utf8(bytes)
                .map_err(|_| AdminError::new(ErrorKind::Generic, "Cannot encode configuration"))?;
            Ok(Outcome::ok(
                json!({
                    "revision": response.revision,
                    "config": response.config,
                    "secrets": true
                }),
                text,
            ))
        }
    }
}
