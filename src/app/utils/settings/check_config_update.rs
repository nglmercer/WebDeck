//! Port of `app/utils/settings/check_config_update.py`.

use serde_json::Value;

use crate::app::buttons::usage::gpu::canonical_gpu_method;
use crate::app::utils::{logger::log, settings::get_config::config_dir, working_dir};

/// Port of `check_config_update`.
pub fn check_config_update(config: Value) -> Value {
    let mut config = check_config_hyphen_case(config);

    rename_key(&mut config, "front", "black_theme", "dark_theme");
    rename_key(
        &mut config,
        "settings",
        "open_settings_in_browser",
        "open_settings_in_integrated_browser",
    );

    // Canonicalize legacy Python-era `gpu_method` values (`pynvml`/`GPUtil`
    // were the old NVML bindings); the UI saves the NVML names now.
    // Drop the Flask-era server keys: the axum backend never reads them
    // (gone from `config_default.json`, no UI writes them anymore).
    if let Some(settings) = config.get_mut("settings").and_then(|s| s.as_object_mut()) {
        let method = settings
            .get("gpu_method")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let canonical = canonical_gpu_method(&method);
        if canonical != method {
            settings.insert(
                "gpu_method".to_string(),
                Value::String(canonical.to_string()),
            );
        }
        for dead in [
            "server",
            "flask_debug",
            "flask_reloader",
            "flask_secret_key",
        ] {
            settings.remove(dead);
        }
    }

    // Lowercase every key in settings.spotify_api.
    if let Some(spotify_api) = config
        .get_mut("settings")
        .and_then(|s| s.get_mut("spotify_api"))
        .and_then(|v| v.as_object_mut())
    {
        let lowered: serde_json::Map<String, Value> = std::mem::take(spotify_api)
            .into_iter()
            .map(|(k, v)| (k.to_lowercase(), v))
            .collect();
        *spotify_api = lowered;
    }

    // Move allowed_networks to settings.allowed_networks.
    if config.get("allowed_networks").is_some() {
        if let Some(root) = config.as_object_mut() {
            let moved = root.remove("allowed_networks");
            if let (Some(settings), Some(moved)) = (
                root.get_mut("settings").and_then(|s| s.as_object_mut()),
                moved,
            ) {
                let target = settings
                    .entry("allowed_networks".to_string())
                    .or_insert(Value::Array(Vec::new()));
                if let (Some(target), Value::Array(items)) = (target.as_array_mut(), moved) {
                    target.extend(items);
                }
            }
        }
    }

    let default_path = working_dir::get_base_dir()
        .join("webdeck")
        .join("config_default.json");
    let default_content =
        std::fs::read_to_string(&default_path).expect("Cannot read webdeck/config_default.json");
    let default_config: Value =
        serde_json::from_str(&default_content).expect("Cannot parse webdeck/config_default.json");

    update_config_with_defaults(&mut config, &default_config);

    let config = check_config_booleans(config);
    check_config_themes(config)
}

/// Rename `old` to `new` inside `config[section]` when present
/// (config migration; missing sections and non-object sections are left
/// untouched, like the Python `if old in config[section]` guards).
fn rename_key(config: &mut Value, section: &str, old: &str, new: &str) {
    if config.get(section).and_then(|s| s.get(old)).is_none() {
        return;
    }
    if let Some(map) = config.get_mut(section).and_then(|s| s.as_object_mut()) {
        if let Some(value) = map.remove(old) {
            map.insert(new.to_string(), value);
        }
    }
}

/// Port of the nested `update_config_with_defaults` in `check_config_update`.
fn update_config_with_defaults(config: &mut Value, default_config: &Value) {
    let Some(default_root) = default_config.as_object() else {
        return;
    };
    let Some(root) = config.as_object_mut() else {
        return;
    };
    for (section, section_value) in default_root {
        match (root.get_mut(section), section_value) {
            (None, default_value) => {
                root.insert(section.clone(), default_value.clone());
            }
            (Some(existing), Value::Object(defaults)) => {
                if let Some(existing_map) = existing.as_object_mut() {
                    for (key, value) in defaults {
                        match (existing_map.get_mut(key), value) {
                            (None, default_value) => {
                                existing_map.insert(key.clone(), default_value.clone());
                            }
                            (Some(existing_value), Value::Object(_)) if key != "buttons" => {
                                update_config_with_defaults(existing_value, value);
                            }
                            _ => {}
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

/// Port of `check_config_themes`.
pub fn check_config_themes(mut config: Value) -> Value {
    let themes_dir = config_dir().join("themes");
    if !themes_dir.exists() {
        return config;
    }

    let mut themes: Vec<String> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&themes_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.ends_with(".css") {
                themes.push(format!("//{name}"));
            }
        }
    }

    let front = config.get_mut("front").and_then(|f| f.as_object_mut());
    let Some(front) = front else {
        return config;
    };

    if !front.contains_key("themes") {
        front.insert(
            "themes".to_string(),
            Value::Array(themes.into_iter().map(Value::String).collect()),
        );
    } else if let Some(installed) = front.get_mut("themes").and_then(|t| t.as_array_mut()) {
        // New themes installed on disk but missing from config.
        let installed_names: Vec<String> = installed
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();
        let new_themes: Vec<String> = themes
            .into_iter()
            .filter(|theme| {
                !installed_names
                    .iter()
                    .any(|name| theme.ends_with(name.as_str()))
            })
            .collect();
        if !new_themes.is_empty() {
            log().debug(&format!("New themes found: {new_themes:?}"));
            installed.extend(new_themes.into_iter().map(Value::String));
        }
    }

    // Python runs eval() on the themes value to tolerate a stringified list;
    // parse either a JSON list or a Python-repr list ("['//a.css']").
    if let Some(themes_value) = front.get("themes").cloned() {
        if let Some(raw) = themes_value.as_str() {
            if let Some(parsed) = parse_stringified_list(raw) {
                front.insert("themes".to_string(), Value::Array(parsed));
            }
        }
    }

    // Drop themes whose file no longer exists.
    if let Some(installed) = front.get_mut("themes").and_then(|t| t.as_array_mut()) {
        installed.retain(|theme| {
            let file = theme.as_str().unwrap_or("").replace("//", "");
            themes_dir.join(file).is_file()
        });

        // Remove duplicates (keep first occurrence, re-inserted at the front
        // like Python).
        let mut seen: Vec<String> = Vec::new();
        let mut deduped: Vec<Value> = Vec::new();
        for theme in installed.iter() {
            let name = theme.as_str().unwrap_or("").replace("//", "");
            if !seen.contains(&name) {
                seen.push(name.clone());
                deduped.push(Value::String(format!("//{name}")));
            }
        }
        // Python moves each duplicate to index 0; net effect for the common
        // cases equals first-occurrence-wins with `//` prefix normalized.
        *installed = deduped;
    }

    config
}

/// Parse a stringified theme list: JSON first, then Python-repr fallback.
fn parse_stringified_list(raw: &str) -> Option<Vec<Value>> {
    if let Ok(Value::Array(items)) = serde_json::from_str::<Value>(raw) {
        return Some(items);
    }
    let trimmed = raw.trim().trim_start_matches('[').trim_end_matches(']');
    if trimmed.trim().is_empty() {
        return Some(Vec::new());
    }
    Some(
        trimmed
            .split(',')
            .map(|item| {
                Value::String(
                    item.trim()
                        .trim_matches(|c| c == '\'' || c == '"')
                        .to_string(),
                )
            })
            .collect(),
    )
}

/// Port of `check_config_booleans`.
pub fn check_config_booleans(mut config: Value) -> Value {
    fn normalize(value: &mut Value) {
        if let Some(text) = value.as_str() {
            if text.eq_ignore_ascii_case("true") {
                *value = Value::Bool(true);
            } else if text.eq_ignore_ascii_case("false") {
                *value = Value::Bool(false);
            }
        }
    }

    if let Some(root) = config.as_object_mut() {
        for settings in root.values_mut() {
            if let Some(map) = settings.as_object_mut() {
                for value in map.values_mut() {
                    if value.is_object() {
                        if let Some(inner) = value.as_object_mut() {
                            for setting in inner.values_mut() {
                                normalize(setting);
                            }
                        }
                    } else {
                        normalize(value);
                    }
                }
            }
        }
    }
    config
}

/// Port of `check_config_hyphen_case`.
pub fn check_config_hyphen_case(config: Value) -> Value {
    let mut new_config = serde_json::Map::new();
    if let Some(root) = config.as_object() {
        for (category, settings) in root {
            let new_category = category.replace('-', "_");
            match settings.as_object() {
                None => {
                    new_config.insert(new_category, settings.clone());
                }
                Some(map) => {
                    let mut new_settings = serde_json::Map::new();
                    for (key, value) in map {
                        let new_key = key.replace('-', "_");
                        match value.as_object() {
                            Some(inner) => {
                                let new_value: serde_json::Map<String, Value> = inner
                                    .iter()
                                    .map(|(k, v)| (k.replace('-', "_"), v.clone()))
                                    .collect();
                                new_settings.insert(new_key, Value::Object(new_value));
                            }
                            None => {
                                new_settings.insert(new_key, value.clone());
                            }
                        }
                    }
                    new_config.insert(new_category, Value::Object(new_settings));
                }
            }
        }
    }
    let mut new_config = Value::Object(new_config);

    // Update background-color to background_color in buttons.
    if let Some(buttons) = new_config
        .get_mut("front")
        .and_then(|f| f.get_mut("buttons"))
        .and_then(|b| b.as_object_mut())
    {
        for page_content in buttons.values_mut() {
            if let Some(list) = page_content.as_array_mut() {
                for button in list.iter_mut() {
                    if let Some(map) = button.as_object_mut() {
                        if let Some(value) = map.remove("background-color") {
                            map.insert("background_color".to_string(), value);
                        }
                    }
                }
            }
        }
    }

    new_config
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn hyphen_case_and_booleans() {
        let config = json!({
            "front": {"background-color": "red", "enabled": "true"},
            "settings": {"nested": {"flag": "FALSE"}}
        });
        let config = check_config_hyphen_case(config);
        assert_eq!(config["front"]["background_color"], json!("red"));
        let config = check_config_booleans(config);
        assert_eq!(config["front"]["enabled"], json!(true));
        assert_eq!(config["settings"]["nested"]["flag"], json!(false));
    }

    #[test]
    fn renames_legacy_keys() {
        // check_config_themes early-returns without .config/themes on disk.
        let config = json!({
            "front": {"black_theme": true, "buttons": {}},
            "settings": {"open_settings_in_browser": false, "spotify_api": {"ClientID": "x"}},
            "allowed_networks": ["10.0.0.0/8"]
        });
        // Run only the pure parts (full check needs webdeck/config_default.json
        // which exists in the repo, so run the whole thing from package root).
        let config = check_config_update(config);
        assert_eq!(config["front"]["dark_theme"], json!(true));
        assert!(config["front"].get("black_theme").is_none());
        assert!(config["settings"]
            .get("open_settings_in_integrated_browser")
            .is_some());
        assert!(config["settings"]["spotify_api"].get("clientid").is_some());
        assert!(config["settings"]["allowed_networks"]
            .as_array()
            .map(|a| !a.is_empty())
            .unwrap_or(false));
    }

    #[test]
    fn migrates_legacy_gpu_and_drops_flask_keys() {
        let config = json!({
            "front": {"buttons": {}},
            "settings": {
                "gpu_method": "nvidia (GPUtil)",
                "server": "flask",
                "flask_debug": true,
                "flask_reloader": false,
                "flask_secret_key": "secret",
            }
        });
        let config = check_config_update(config);
        assert_eq!(
            config["settings"]["gpu_method"],
            json!("nvidia (NVML detailed)")
        );
        for dead in [
            "server",
            "flask_debug",
            "flask_reloader",
            "flask_secret_key",
        ] {
            assert!(
                config["settings"].get(dead).is_none(),
                "{dead} should be dropped"
            );
        }
    }
}
