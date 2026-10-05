use super::client::{MutationReport, WebDeckAdminClient};
use super::error::{AdminError, ErrorKind, Result};
use super::output::Outcome;
use super::{merge_patch, read_value, report_value};
use crate::contracts::{Config, Folder};
use crate::domain;
use serde_json::{json, Map, Value};

fn folder_index(config: &Config, id: &str) -> Option<usize> {
    config.layout.folders.iter().position(|f| f.id == id)
}

fn typed_folder(value: Value) -> Result<Folder> {
    let folder: Folder = serde_json::from_value(value).map_err(|e| {
        AdminError::with_details(
            ErrorKind::Validation,
            "Invalid folder shape",
            json!({"reason": e.to_string()}),
        )
    })?;
    domain::validate(
        "Folder",
        &serde_json::to_value(&folder).unwrap_or(Value::Null),
    )
    .map_err(AdminError::from_domain)?;
    Ok(folder)
}

fn base_folder(file: Option<&str>, id: Option<&str>, label: Option<&str>) -> Result<Value> {
    let mut object = match file {
        Some(path) => {
            let value = read_value(Some(path))?;
            value.as_object().cloned().ok_or_else(|| {
                AdminError::invalid_arguments("Folder input must be a JSON object")
            })?
        }
        None => Map::new(),
    };
    if let Some(id) = id {
        if let Some(existing) = object.get("id").and_then(|v| v.as_str()) {
            if existing != id {
                return Err(AdminError::invalid_arguments(format!(
                    "Conflicting folder id: file has '{existing}', --id has '{id}'"
                )));
            }
        }
        object.insert("id".into(), json!(id));
    }
    if let Some(label) = label {
        object.insert("label".into(), json!(label));
    }
    let folder_id = object
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AdminError::invalid_arguments("Folder requires an id (--file or --id)"))?
        .to_string();
    object
        .entry("label")
        .or_insert_with(|| json!(folder_id.clone()));
    object.entry("buttons").or_insert_with(|| json!([]));
    object.entry("extensions").or_insert_with(|| json!({}));
    Ok(Value::Object(object))
}

fn report(
    operation: &str,
    revision: u64,
    previous: Option<u64>,
    applied: bool,
    dry_run: bool,
    id: &str,
) -> Value {
    report_value(
        &MutationReport {
            operation: operation.into(),
            revision,
            previous_revision: previous,
            applied,
            dry_run,
        },
        json!({"id": id}),
    )
}

pub async fn list(client: &WebDeckAdminClient) -> Result<Outcome> {
    let response = client.get_config().await?;
    let folders: Vec<Value> = response
        .config
        .layout
        .folders
        .iter()
        .map(|f| {
            json!({
                "id": f.id,
                "label": f.label,
                "button_count": f.buttons.len()
            })
        })
        .collect();
    let count = folders.len();
    let rows = folders
        .iter()
        .map(|b| {
            format!(
                "{}\t{}\t{} button(s)",
                b["id"].as_str().unwrap_or(""),
                b["label"].as_str().unwrap_or(""),
                b["button_count"]
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    Ok(Outcome::ok(
        json!({"revision": response.revision, "folders": folders}),
        format!(
            "{count} folder(s) at revision {}.\n{rows}",
            response.revision
        ),
    ))
}

pub async fn create(
    client: &WebDeckAdminClient,
    file: Option<&str>,
    id: Option<&str>,
    label: Option<&str>,
    revision: Option<u64>,
    dry_run: bool,
) -> Result<Outcome> {
    let folder = typed_folder(base_folder(file, id, label)?)?;
    let current = client.get_config().await?;
    if let Some(expected) = revision {
        client.ensure_revision(Some(expected), current.revision)?;
    }
    let expected = revision.unwrap_or(current.revision);
    if folder_index(&current.config, &folder.id).is_some() {
        return Err(AdminError::with_details(
            ErrorKind::RevisionConflict,
            format!("Folder '{}' already exists", folder.id),
            json!({"id": folder.id}),
        ));
    }
    let id = folder.id.clone();
    if dry_run {
        return Ok(Outcome::ok(
            report("folder_create", current.revision, None, false, true, &id),
            format!(
                "Dry run: would create folder '{id}' at revision {}.",
                current.revision
            ),
        ));
    }
    let response = match client.create_folder(expected, folder).await {
        Ok(response) => response,
        Err(error) => return Err(client.enrich_conflict(error, expected).await),
    };
    Ok(Outcome::ok(
        report(
            "folder_create",
            response.revision,
            Some(expected),
            true,
            false,
            &id,
        ),
        format!(
            "Created folder '{id}' (revision {expected} -> {}).",
            response.revision
        ),
    ))
}

pub async fn update(
    client: &WebDeckAdminClient,
    file: Option<&str>,
    id: Option<&str>,
    label: Option<&str>,
    revision: Option<u64>,
    dry_run: bool,
) -> Result<Outcome> {
    let patch = base_patch(file, id, label)?;
    let folder_id = patch
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AdminError::invalid_arguments("folder update requires --file or --id"))?
        .to_string();
    patch_id_unchanged(&patch, &folder_id)?;
    let current = client.get_config().await?;
    if let Some(expected) = revision {
        client.ensure_revision(Some(expected), current.revision)?;
    }
    let expected = revision.unwrap_or(current.revision);
    let index = folder_index(&current.config, &folder_id).ok_or_else(|| {
        AdminError::with_details(
            ErrorKind::NotFound,
            format!("Folder '{folder_id}' does not exist"),
            json!({"id": folder_id}),
        )
    })?;
    let existing_value =
        serde_json::to_value(&current.config.layout.folders[index]).unwrap_or(Value::Null);
    let merged = typed_folder(merge_patch(&existing_value, &Value::Object(patch)))?;
    if merged == current.config.layout.folders[index] {
        return Ok(Outcome::ok(
            report(
                "folder_update",
                current.revision,
                None,
                false,
                false,
                &folder_id,
            ),
            format!(
                "Folder '{folder_id}' is unchanged at revision {}.",
                current.revision
            ),
        ));
    }
    if dry_run {
        return Ok(Outcome::ok(
            report(
                "folder_update",
                current.revision,
                None,
                false,
                true,
                &folder_id,
            ),
            format!(
                "Dry run: would update folder '{folder_id}' at revision {}.",
                current.revision
            ),
        ));
    }
    let mut config = current.config.clone();
    config.layout.folders[index] = merged;
    let response = match client.put_config(expected, config).await {
        Ok(response) => response,
        Err(error) => return Err(client.enrich_conflict(error, expected).await),
    };
    Ok(Outcome::ok(
        report(
            "folder_update",
            response.revision,
            Some(expected),
            true,
            false,
            &folder_id,
        ),
        format!(
            "Updated folder '{folder_id}' (revision {expected} -> {}).",
            response.revision
        ),
    ))
}

pub async fn delete(
    client: &WebDeckAdminClient,
    id: &str,
    revision: Option<u64>,
    dry_run: bool,
) -> Result<Outcome> {
    let current = client.get_config().await?;
    if let Some(expected) = revision {
        client.ensure_revision(Some(expected), current.revision)?;
    }
    let expected = revision.unwrap_or(current.revision);
    if folder_index(&current.config, id).is_none() {
        return Err(AdminError::with_details(
            ErrorKind::NotFound,
            format!("Folder '{id}' does not exist"),
            json!({"id": id}),
        ));
    }
    if dry_run {
        return Ok(Outcome::ok(
            report("folder_delete", current.revision, None, false, true, id),
            format!(
                "Dry run: would delete folder '{id}' at revision {}.",
                current.revision
            ),
        ));
    }
    let response = match client.delete_folder(expected, id).await {
        Ok(response) => response,
        Err(error) => return Err(client.enrich_conflict(error, expected).await),
    };
    Ok(Outcome::ok(
        report(
            "folder_delete",
            response.revision,
            Some(expected),
            true,
            false,
            id,
        ),
        format!(
            "Deleted folder '{id}' (revision {expected} -> {}).",
            response.revision
        ),
    ))
}

pub async fn ensure(
    client: &WebDeckAdminClient,
    file: Option<&str>,
    id: Option<&str>,
    label: Option<&str>,
    revision: Option<u64>,
    dry_run: bool,
) -> Result<Outcome> {
    let base = base_folder(file, id, label)?;
    let folder_id = base
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AdminError::invalid_arguments("folder ensure requires --file or --id"))?
        .to_string();
    let mut explicit_label = label.map(str::to_string);
    if explicit_label.is_none() {
        if let Some(path) = file {
            let value = read_value(Some(path))?;
            explicit_label = value
                .get("label")
                .and_then(|v| v.as_str())
                .map(str::to_string);
        }
    }
    let current = client.get_config().await?;
    if let Some(expected) = revision {
        client.ensure_revision(Some(expected), current.revision)?;
    }
    let expected = revision.unwrap_or(current.revision);
    let Some(index) = folder_index(&current.config, &folder_id) else {
        let folder = typed_folder(base)?;
        if dry_run {
            return Ok(Outcome::ok(
                report(
                    "folder_ensure",
                    current.revision,
                    None,
                    false,
                    true,
                    &folder_id,
                ),
                format!(
                    "Dry run: would create folder '{folder_id}' at revision {}.",
                    current.revision
                ),
            ));
        }
        let response = match client.create_folder(expected, folder).await {
            Ok(response) => response,
            Err(error) => return Err(client.enrich_conflict(error, expected).await),
        };
        return Ok(Outcome::ok(
            report(
                "folder_ensure",
                response.revision,
                Some(expected),
                true,
                false,
                &folder_id,
            ),
            format!(
                "Created folder '{folder_id}' (revision {expected} -> {}).",
                response.revision
            ),
        ));
    };
    let existing = &current.config.layout.folders[index];
    let desired_label = explicit_label.unwrap_or_else(|| existing.label.clone());
    if existing.label == desired_label {
        return Ok(Outcome::ok(
            report(
                "folder_ensure",
                current.revision,
                None,
                false,
                false,
                &folder_id,
            ),
            format!(
                "Folder '{folder_id}' already ensured at revision {}.",
                current.revision
            ),
        ));
    }
    if dry_run {
        return Ok(Outcome::ok(
            report(
                "folder_ensure",
                current.revision,
                None,
                false,
                true,
                &folder_id,
            ),
            format!(
                "Dry run: would update folder '{folder_id}' label at revision {}.",
                current.revision
            ),
        ));
    }
    let mut config = current.config.clone();
    config.layout.folders[index].label = desired_label;
    let response = match client.put_config(expected, config).await {
        Ok(response) => response,
        Err(error) => return Err(client.enrich_conflict(error, expected).await),
    };
    Ok(Outcome::ok(
        report(
            "folder_ensure",
            response.revision,
            Some(expected),
            true,
            false,
            &folder_id,
        ),
        format!(
            "Updated folder '{folder_id}' (revision {expected} -> {}).",
            response.revision
        ),
    ))
}

fn base_patch(
    file: Option<&str>,
    id: Option<&str>,
    label: Option<&str>,
) -> Result<Map<String, Value>> {
    let mut patch = match file {
        Some(path) => {
            let value = read_value(Some(path))?;
            value.as_object().cloned().ok_or_else(|| {
                AdminError::invalid_arguments("Folder input must be a JSON object")
            })?
        }
        None => Map::new(),
    };
    if let Some(id) = id {
        if let Some(existing) = patch.get("id").and_then(|v| v.as_str()) {
            if existing != id {
                return Err(AdminError::invalid_arguments(format!(
                    "Conflicting folder id: file has '{existing}', --id has '{id}'"
                )));
            }
        }
        patch.insert("id".into(), json!(id));
    }
    if let Some(label) = label {
        patch.insert("label".into(), json!(label));
    }
    Ok(patch)
}

fn patch_id_unchanged(patch: &Map<String, Value>, id: &str) -> Result<()> {
    match patch.get("id").and_then(|v| v.as_str()) {
        Some(value) if value == id => Ok(()),
        Some(value) => Err(AdminError::invalid_arguments(format!(
            "Folder id cannot be changed ('{value}' != '{id}')"
        ))),
        None => Err(AdminError::invalid_arguments(
            "folder update requires --file or --id",
        )),
    }
}
