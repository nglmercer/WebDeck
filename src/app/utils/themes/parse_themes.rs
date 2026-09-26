//! Port of `app/utils/themes/parse_themes.py`.

use std::collections::HashMap;

use serde_json::Value;

use crate::app::utils::languages::text;

/// Port of `parse_css_file`.
///
/// Reads `key=value` header lines up to the `/*-------…​*/` marker, then fills
/// defaults for the known metadata keys (same order and fallbacks as Python).
pub fn parse_css_file(css_file_path: &str) -> HashMap<String, Value> {
    let mut css_data: HashMap<String, Value> = HashMap::new();

    if let Ok(content) = std::fs::read_to_string(css_file_path) {
        for line in content.lines() {
            let line = line.trim();
            if line.replace(' ', "").starts_with("/*-------") {
                break;
            }
            if line.contains('=') {
                if let Some((key, value)) = line.split_once('=') {
                    css_data.insert(
                        key.trim().to_lowercase(),
                        Value::String(value.trim().to_string()),
                    );
                }
            }
        }
    }

    let not_specified = text(Some("not_specified"), None);
    for info in [
        "theme-icon",
        "theme-name",
        "theme-description",
        "theme-author-github",
        "page-preview",
    ] {
        css_data
            .entry(info.to_string())
            .or_insert(Value::String(not_specified.clone()));
    }

    let is_missing = |data: &HashMap<String, Value>, key: &str| {
        data.get(key).and_then(|v| v.as_str()) == Some(not_specified.as_str())
    };
    let icon_missing = is_missing(&css_data, "theme-icon");
    let name_missing = is_missing(&css_data, "theme-name");
    let description_missing = is_missing(&css_data, "theme-description");
    let preview_missing = is_missing(&css_data, "page-preview");

    if icon_missing {
        let fallback = css_data
            .get("theme-icon")
            .cloned()
            .filter(|v| !v.as_str().map(|s| s.is_empty()).unwrap_or(true))
            .or_else(|| css_data.get("theme-logo").cloned())
            .unwrap_or(Value::String(String::new()));
        css_data.insert("theme-icon".to_string(), fallback);
    }
    if name_missing {
        let basename = std::path::Path::new(css_file_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(css_file_path)
            .to_string();
        css_data.insert("theme-name".to_string(), Value::String(basename));
    }
    if description_missing {
        css_data.insert(
            "theme-description".to_string(),
            Value::String(css_file_path.to_string()),
        );
    }
    if preview_missing {
        css_data.insert(
            "page-preview".to_string(),
            Value::Array(vec![Value::String("all".to_string())]),
        );
    }

    css_data
}

/// Port of `parse_themes`.
pub fn parse_themes() -> HashMap<String, HashMap<String, Value>> {
    let mut parsed_themes: HashMap<String, HashMap<String, Value>> = HashMap::new();
    if let Ok(entries) = std::fs::read_dir(".config/themes/") {
        for entry in entries.flatten() {
            let file_name = entry.file_name().to_string_lossy().to_string();
            if file_name.ends_with(".css") {
                parsed_themes.insert(
                    file_name.clone(),
                    parse_css_file(&format!(".config/themes/{file_name}")),
                );
            }
        }
    }

    parsed_themes.insert(
        "static/css/style.css".to_string(),
        parse_css_file("static/css/style.css"),
    );

    parsed_themes
}
