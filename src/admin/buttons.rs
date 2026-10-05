use super::client::{MutationReport, WebDeckAdminClient};
use super::error::{AdminError, ErrorKind, Result};
use super::output::Outcome;
use super::{merge_patch, read_value, report_value};
use crate::contracts::{Button, Config};
use crate::domain;
use serde_json::{json, Map, Value};

fn typed_button(value: Value) -> Result<Button> {
    if let Some(action) = value.get("action") {
        if domain::validate("ButtonAction", action).is_err()
            && domain::validate("Command", action).is_ok()
        {
            return Err(AdminError::with_details(
                ErrorKind::Validation,
                "Button commands require an action wrapper: {\"type\":\"command\",\"command\":{...}}",
                json!({"path": "action", "example": {"type": "command", "command": action}}),
            ));
        }
    }
    let button: Button = serde_json::from_value(value).map_err(|e| {
        AdminError::with_details(
            ErrorKind::Validation,
            "Invalid button shape",
            json!({"reason": e.to_string()}),
        )
    })?;
    domain::validate(
        "Button",
        &serde_json::to_value(&button).unwrap_or(Value::Null),
    )
    .map_err(AdminError::from_domain)?;
    Ok(button)
}

fn patch_value(file: Option<&str>) -> Result<Map<String, Value>> {
    match file {
        Some(path) => {
            let value = read_value(Some(path))?;
            value
                .as_object()
                .cloned()
                .ok_or_else(|| AdminError::invalid_arguments("Button input must be a JSON object"))
        }
        None => Ok(Map::new()),
    }
}

fn with_defaults(mut object: Map<String, Value>) -> Map<String, Value> {
    let default_label = object.get("id").cloned().unwrap_or(Value::Null);
    object.entry("label").or_insert(default_label);
    object.entry("icon").or_insert_with(|| json!(""));
    object.entry("color").or_insert_with(|| json!(""));
    object
        .entry("action")
        .or_insert_with(|| json!({"type": "none"}));
    object.entry("extensions").or_insert_with(|| json!({}));
    object
}

fn button_id_in_use(config: &Config, id: &str) -> bool {
    config.layout.folders.iter().any(|f| f.id == id)
        || config
            .layout
            .folders
            .iter()
            .any(|f| f.buttons.iter().any(|b| b.id == id))
}

fn report(
    operation: &str,
    revision: u64,
    previous: Option<u64>,
    applied: bool,
    dry_run: bool,
    id: &str,
    folder: &str,
) -> Value {
    report_value(
        &MutationReport {
            operation: operation.into(),
            revision,
            previous_revision: previous,
            applied,
            dry_run,
        },
        json!({"id": id, "folder": folder}),
    )
}

pub async fn list(client: &WebDeckAdminClient, folder: Option<&str>) -> Result<Outcome> {
    let response = client.get_config().await?;
    if let Some(folder_id) = folder {
        if response
            .config
            .layout
            .folders
            .iter()
            .all(|f| f.id != folder_id)
        {
            return Err(AdminError::with_details(
                ErrorKind::NotFound,
                format!("Folder '{folder_id}' does not exist"),
                json!({"id": folder_id}),
            ));
        }
    }
    let mut buttons = Vec::new();
    for f in &response.config.layout.folders {
        if folder.is_some_and(|id| id != f.id) {
            continue;
        }
        for b in &f.buttons {
            buttons.push(json!({
                "id": b.id,
                "label": b.label,
                "folder": f.id,
                "action": serde_json::to_value(&b.action).unwrap_or(Value::Null)
            }));
        }
    }
    let count = buttons.len();
    let rows = buttons
        .iter()
        .map(|b| {
            format!(
                "{}\t{}\t{}\t{}",
                b["id"].as_str().unwrap_or(""),
                b["label"].as_str().unwrap_or(""),
                b["folder"].as_str().unwrap_or(""),
                b["action"]
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    Ok(Outcome::ok(
        json!({"revision": response.revision, "buttons": buttons}),
        format!(
            "{count} button(s) at revision {}.\n{rows}",
            response.revision
        ),
    ))
}

pub async fn create(
    client: &WebDeckAdminClient,
    folder_id: &str,
    file: Option<&str>,
    revision: Option<u64>,
    dry_run: bool,
) -> Result<Outcome> {
    let object = patch_value(file)?;
    let id = object
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AdminError::invalid_arguments("Button requires an id in --file"))?
        .to_string();
    let button = typed_button(Value::Object(with_defaults(object)))?;
    let current = client.get_config().await?;
    if let Some(expected) = revision {
        client.ensure_revision(Some(expected), current.revision)?;
    }
    let expected = revision.unwrap_or(current.revision);
    if current
        .config
        .layout
        .folders
        .iter()
        .all(|f| f.id != folder_id)
    {
        return Err(AdminError::with_details(
            ErrorKind::NotFound,
            format!("Folder '{folder_id}' does not exist"),
            json!({"id": folder_id}),
        ));
    }
    if button_id_in_use(&current.config, &id) {
        return Err(AdminError::with_details(
            ErrorKind::RevisionConflict,
            format!("Button id '{id}' is already in use"),
            json!({"id": id}),
        ));
    }
    if dry_run {
        return Ok(Outcome::ok(
            report(
                "button_create",
                current.revision,
                None,
                false,
                true,
                &id,
                folder_id,
            ),
            format!(
                "Dry run: would create button '{id}' in '{folder_id}' at revision {}.",
                current.revision
            ),
        ));
    }
    let response = match client.create_button(folder_id, expected, button).await {
        Ok(response) => response,
        Err(error) => return Err(client.enrich_conflict(error, expected).await),
    };
    Ok(Outcome::ok(
        report(
            "button_create",
            response.revision,
            Some(expected),
            true,
            false,
            &id,
            folder_id,
        ),
        format!(
            "Created button '{id}' in '{folder_id}' (revision {expected} -> {}).",
            response.revision
        ),
    ))
}

pub async fn update(
    client: &WebDeckAdminClient,
    folder_id: &str,
    file: Option<&str>,
    revision: Option<u64>,
    dry_run: bool,
) -> Result<Outcome> {
    let patch = patch_value(file)?;
    let id = patch
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AdminError::invalid_arguments("Button update requires --file with an id"))?
        .to_string();
    let current = client.get_config().await?;
    if let Some(expected) = revision {
        client.ensure_revision(Some(expected), current.revision)?;
    }
    let expected = revision.unwrap_or(current.revision);
    let folder = current
        .config
        .layout
        .folders
        .iter()
        .find(|f| f.id == folder_id)
        .ok_or_else(|| {
            AdminError::with_details(
                ErrorKind::NotFound,
                format!("Folder '{folder_id}' does not exist"),
                json!({"id": folder_id}),
            )
        })?;
    let existing = folder.buttons.iter().find(|b| b.id == id).ok_or_else(|| {
        AdminError::with_details(
            ErrorKind::NotFound,
            format!("Button '{id}' does not exist in '{folder_id}'"),
            json!({"id": id, "folder": folder_id}),
        )
    })?;
    let existing_value = serde_json::to_value(existing).unwrap_or(Value::Null);
    let merged = typed_button(merge_patch(&existing_value, &Value::Object(patch)))?;
    if &merged == existing {
        return Ok(Outcome::ok(
            report(
                "button_update",
                current.revision,
                None,
                false,
                false,
                &id,
                folder_id,
            ),
            format!(
                "Button '{id}' is unchanged at revision {}.",
                current.revision
            ),
        ));
    }
    if dry_run {
        return Ok(Outcome::ok(
            report(
                "button_update",
                current.revision,
                None,
                false,
                true,
                &id,
                folder_id,
            ),
            format!(
                "Dry run: would update button '{id}' in '{folder_id}' at revision {}.",
                current.revision
            ),
        ));
    }
    let response = match client.update_button(folder_id, expected, merged).await {
        Ok(response) => response,
        Err(error) => return Err(client.enrich_conflict(error, expected).await),
    };
    Ok(Outcome::ok(
        report(
            "button_update",
            response.revision,
            Some(expected),
            true,
            false,
            &id,
            folder_id,
        ),
        format!(
            "Updated button '{id}' in '{folder_id}' (revision {expected} -> {}).",
            response.revision
        ),
    ))
}

pub async fn delete(
    client: &WebDeckAdminClient,
    folder_id: &str,
    id: &str,
    revision: Option<u64>,
    dry_run: bool,
) -> Result<Outcome> {
    let current = client.get_config().await?;
    if let Some(expected) = revision {
        client.ensure_revision(Some(expected), current.revision)?;
    }
    let expected = revision.unwrap_or(current.revision);
    let folder = current
        .config
        .layout
        .folders
        .iter()
        .find(|f| f.id == folder_id)
        .ok_or_else(|| {
            AdminError::with_details(
                ErrorKind::NotFound,
                format!("Folder '{folder_id}' does not exist"),
                json!({"id": folder_id}),
            )
        })?;
    if !folder.buttons.iter().any(|b| b.id == id) {
        return Err(AdminError::with_details(
            ErrorKind::NotFound,
            format!("Button '{id}' does not exist in '{folder_id}'"),
            json!({"id": id, "folder": folder_id}),
        ));
    }
    if dry_run {
        return Ok(Outcome::ok(
            report(
                "button_delete",
                current.revision,
                None,
                false,
                true,
                id,
                folder_id,
            ),
            format!(
                "Dry run: would delete button '{id}' from '{folder_id}' at revision {}.",
                current.revision
            ),
        ));
    }
    let response = match client.delete_button(folder_id, id, expected).await {
        Ok(response) => response,
        Err(error) => return Err(client.enrich_conflict(error, expected).await),
    };
    Ok(Outcome::ok(
        report(
            "button_delete",
            response.revision,
            Some(expected),
            true,
            false,
            id,
            folder_id,
        ),
        format!(
            "Deleted button '{id}' from '{folder_id}' (revision {expected} -> {}).",
            response.revision
        ),
    ))
}

pub async fn ensure(
    client: &WebDeckAdminClient,
    folder_id: &str,
    file: Option<&str>,
    id: Option<&str>,
    revision: Option<u64>,
    dry_run: bool,
) -> Result<Outcome> {
    let mut patch = patch_value(file)?;
    if let Some(id) = id {
        if let Some(existing) = patch.get("id").and_then(|v| v.as_str()) {
            if existing != id {
                return Err(AdminError::invalid_arguments(format!(
                    "Conflicting button id: file has '{existing}', --id has '{id}'"
                )));
            }
        }
        patch.insert("id".into(), json!(id));
    }
    let button_id = patch
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AdminError::invalid_arguments("button ensure requires --file or --id"))?
        .to_string();
    let current = client.get_config().await?;
    if let Some(expected) = revision {
        client.ensure_revision(Some(expected), current.revision)?;
    }
    let expected = revision.unwrap_or(current.revision);
    if current
        .config
        .layout
        .folders
        .iter()
        .all(|f| f.id != folder_id)
    {
        return Err(AdminError::with_details(
            ErrorKind::NotFound,
            format!("Folder '{folder_id}' does not exist"),
            json!({"id": folder_id}),
        ));
    }
    let existing = current
        .config
        .layout
        .folders
        .iter()
        .find(|f| f.id == folder_id)
        .and_then(|f| f.buttons.iter().find(|b| b.id == button_id));
    let Some(existing) = existing else {
        let mut object = with_defaults(patch);
        object.insert("id".into(), json!(button_id.clone()));
        let button = typed_button(Value::Object(object))?;
        if button_id_in_use(&current.config, &button_id) {
            return Err(AdminError::with_details(
                ErrorKind::RevisionConflict,
                format!("Button id '{button_id}' is already in use"),
                json!({"id": button_id}),
            ));
        }
        if dry_run {
            return Ok(Outcome::ok(
                report(
                    "button_ensure",
                    current.revision,
                    None,
                    false,
                    true,
                    &button_id,
                    folder_id,
                ),
                format!(
                    "Dry run: would create button '{button_id}' in '{folder_id}' at revision {}.",
                    current.revision
                ),
            ));
        }
        let response = match client.create_button(folder_id, expected, button).await {
            Ok(response) => response,
            Err(error) => return Err(client.enrich_conflict(error, expected).await),
        };
        return Ok(Outcome::ok(
            report(
                "button_ensure",
                response.revision,
                Some(expected),
                true,
                false,
                &button_id,
                folder_id,
            ),
            format!(
                "Created button '{button_id}' in '{folder_id}' (revision {expected} -> {}).",
                response.revision
            ),
        ));
    };
    let existing_value = serde_json::to_value(existing).unwrap_or(Value::Null);
    let merged = typed_button(merge_patch(&existing_value, &Value::Object(patch)))?;
    if &merged == existing {
        return Ok(Outcome::ok(
            report(
                "button_ensure",
                current.revision,
                None,
                false,
                false,
                &button_id,
                folder_id,
            ),
            format!(
                "Button '{button_id}' already ensured at revision {}.",
                current.revision
            ),
        ));
    }
    if dry_run {
        return Ok(Outcome::ok(
            report(
                "button_ensure",
                current.revision,
                None,
                false,
                true,
                &button_id,
                folder_id,
            ),
            format!(
                "Dry run: would update button '{button_id}' in '{folder_id}' at revision {}.",
                current.revision
            ),
        ));
    }
    let response = match client.update_button(folder_id, expected, merged).await {
        Ok(response) => response,
        Err(error) => return Err(client.enrich_conflict(error, expected).await),
    };
    Ok(Outcome::ok(
        report(
            "button_ensure",
            response.revision,
            Some(expected),
            true,
            false,
            &button_id,
            folder_id,
        ),
        format!(
            "Updated button '{button_id}' in '{folder_id}' (revision {expected} -> {}).",
            response.revision
        ),
    ))
}
