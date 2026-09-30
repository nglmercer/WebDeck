//! Theme discovery for an already validated configuration. No version conversion.
use crate::app::utils::{logger::log, settings::get_config::config_dir};
use serde_json::Value;

pub fn check_config_update(config: Value) -> Value {
    crate::domain::config::ConfigDocument::validate(config.clone())
        .expect("Configuration must be validated before theme discovery");
    check_config_themes(config)
}

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
                    .any(|name| theme.trim_start_matches("//") == name.trim_start_matches("//"))
            })
            .collect();
        if !new_themes.is_empty() {
            log().debug(&format!("New themes found: {new_themes:?}"));
            installed.extend(new_themes.into_iter().map(Value::String));
        }
    }

    // Drop themes whose file no longer exists.
    if let Some(installed) = front.get_mut("themes").and_then(|t| t.as_array_mut()) {
        installed.retain(|theme| {
            let file = theme.as_str().unwrap_or("").replace("//", "");
            themes_dir.join(file).is_file()
        });

        // Preserve the first entry including its enabled/disabled state.
        let mut seen: Vec<String> = Vec::new();
        let mut deduped: Vec<Value> = Vec::new();
        for theme in installed.iter() {
            let name = theme.as_str().unwrap_or("").replace("//", "");
            if !seen.contains(&name) {
                seen.push(name.clone());
                deduped.push(theme.clone());
            }
        }
        // Normalizing every entry to // silently disabled themes after save.
        *installed = deduped;
    }

    config
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn discovery_does_not_rename_coerce_or_merge_user_data() {
        let config = json!({"schema_version":2,"front":{"buttons":{}},"settings":{"extension":{"flag":"FALSE","hyphen-key":"keep"}}});
        let discovered = check_config_update(config.clone());
        assert_eq!(discovered["settings"], config["settings"]);
        assert!(discovered["settings"].get("auto_updates").is_none());
    }
}
