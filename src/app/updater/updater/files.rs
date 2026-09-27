//! Installed-file maintenance (extracted from `updater.rs`).
//!
//! Post-update `version.json` reconciliation: deleted files, moved/renamed
//! files, and folder staging.

use serde_json::Value;

use super::versions::compare_versions;
use crate::app::utils::{logger::updater_log, working_dir::get_base_dir};

/// Port of `check_files`.
pub fn check_files() {
    let wd_dir = get_base_dir();
    let version_path = wd_dir.join("webdeck").join("version.json");
    let temp_json_path = wd_dir.join("temp.json");

    let current_version = std::fs::read_to_string(&version_path)
        .ok()
        .and_then(|content| serde_json::from_str::<Value>(&content).ok())
        .and_then(|versions| {
            versions
                .get("versions")?
                .get(0)?
                .get("version")?
                .as_str()
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| "0.0.0".to_string());

    let mut temp_json: Value = std::fs::read_to_string(&temp_json_path)
        .ok()
        .and_then(|content| serde_json::from_str(&content).ok())
        .unwrap_or(Value::Object(serde_json::Map::new()));
    if temp_json.get("checked-versions").is_none() {
        temp_json["checked-versions"] = Value::Array(Vec::new());
    }

    let versions: Vec<Value> = std::fs::read_to_string(&version_path)
        .ok()
        .and_then(|content| serde_json::from_str::<Value>(&content).ok())
        .and_then(|v| v.get("versions").cloned())
        .and_then(|v| v.as_array().cloned())
        .unwrap_or_default();

    for version in versions.iter().rev() {
        let version_name = version
            .get("version")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let already_checked = temp_json
            .get("checked-versions")
            .and_then(|v| v.as_array())
            .map(|a| a.iter().any(|v| v.as_str() == Some(version_name.as_str())))
            .unwrap_or(false);
        if already_checked {
            continue;
        }
        if let Some(checked) = temp_json
            .get_mut("checked-versions")
            .and_then(|v| v.as_array_mut())
        {
            checked.push(Value::String(version_name));
        }

        // Handle deleted files.
        if let Some(deleted) = version.get("deleted_files").and_then(|v| v.as_array()) {
            for file_entry in deleted {
                let (file_to_delete, update_limit) = match file_entry {
                    Value::String(name) => (name.clone(), "99.99.99".to_string()),
                    Value::Array(pair) => (
                        pair.get(0)
                            .and_then(|v| v.as_str())
                            .unwrap_or_default()
                            .to_string(),
                        pair.get(1)
                            .and_then(|v| v.as_str())
                            .unwrap_or("99.99.99")
                            .to_string(),
                    ),
                    _ => continue,
                };
                if compare_versions(&update_limit, &current_version) > 0 {
                    let path = wd_dir.join(&file_to_delete);
                    match std::fs::remove_file(&path) {
                        Ok(()) => {
                            updater_log().info(&format!("UPDATER: Deleted {}", path.display()))
                        }
                        Err(e) => updater_log().exception(
                            &e,
                            Some(&format!("UPDATER: Error deleting {}", path.display())),
                            true,
                            true,
                            true,
                        ),
                    }
                }
            }
        }

        // Handle moved or renamed files.
        let mut files_to_move: Vec<&Value> = Vec::new();
        for key in ["moved_files", "renamed_files"] {
            if let Some(list) = version.get(key).and_then(|v| v.as_array()) {
                files_to_move.extend(list.iter());
            }
        }
        for entry in files_to_move {
            let Some(pair) = entry.as_array() else {
                continue;
            };
            let source = wd_dir.join(pair.get(0).and_then(|v| v.as_str()).unwrap_or_default());
            let destination = wd_dir.join(pair.get(1).and_then(|v| v.as_str()).unwrap_or_default());
            let update_limit = pair.get(2).and_then(|v| v.as_str()).unwrap_or("99.99.99");
            if compare_versions(update_limit, &current_version) > 0 {
                if let Some(parent) = destination.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                match std::fs::rename(&source, &destination) {
                    Ok(()) => updater_log().info(&format!(
                        "Moved {} -> {}",
                        source.display(),
                        destination.display()
                    )),
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                        updater_log().error(&format!("File not found: {}", source.display()));
                    }
                    Err(e) => updater_log().exception(
                        &e,
                        Some(&format!("UPDATER: Error moving {}", source.display())),
                        true,
                        true,
                        true,
                    ),
                }
            }
        }
    }

    let _ = std::fs::write(
        &temp_json_path,
        serde_json::to_string_pretty(&temp_json).unwrap_or_default(),
    );
}

/// Port of `move_folder_content` (progress bar replaced by a summary log).
pub fn move_folder_content(source: &std::path::Path, destination: &std::path::Path) {
    if !destination.exists() {
        let _ = std::fs::create_dir_all(destination);
    }

    let mut elements: Vec<std::path::PathBuf> = Vec::new();
    let mut stack = vec![source.to_path_buf()];
    while let Some(dir) = stack.pop() {
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path.clone());
                }
                elements.push(path);
            }
        }
    }

    let mut moved = 0;
    for element in &elements {
        let Ok(relative) = element.strip_prefix(source) else {
            continue;
        };
        let destination_path = destination.join(relative);
        if element.is_file() {
            if let Some(parent) = destination_path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if std::fs::copy(element, &destination_path).is_ok() {
                moved += 1;
            }
        } else if element.is_dir() {
            let _ = std::fs::create_dir_all(&destination_path);
            moved += 1;
        }
    }
    updater_log().info(&format!("Moving files: {moved}/{} done", elements.len()));
}
