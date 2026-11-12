use super::frontend;
use crate::domain::{Error, Result};
use serde_json::{json, Value};

pub(crate) type TranslationCatalog = std::collections::BTreeMap<String, String>;

fn valid_catalog_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
}

fn decode_catalog(text: &str) -> Option<TranslationCatalog> {
    serde_json::from_str(text).ok()
}

fn read_catalog(root: &std::path::Path, name: &str) -> Option<TranslationCatalog> {
    if !valid_catalog_name(name) {
        return None;
    }
    let catalog = std::fs::read_to_string(root.join(format!("{name}.json")))
        .ok()
        .and_then(|text| decode_catalog(&text));
    #[cfg(webdeck_embedded_frontend)]
    let catalog =
        catalog.or_else(|| frontend::translation(name).and_then(|text| decode_catalog(&text)));
    catalog
}

pub(crate) fn dictionary(language: &str) -> Result<TranslationCatalog> {
    #[cfg(not(webdeck_embedded_frontend))]
    let root = frontend::disk_root("webdeck/translations");
    #[cfg(webdeck_embedded_frontend)]
    let root = std::path::PathBuf::from("webdeck/translations");
    let mut result = read_catalog(&root, "en_US").ok_or_else(Error::execution)?;
    if language != "en_US" {
        if let Some(localized) = read_catalog(&root, language) {
            result.extend(localized);
        }
    }
    Ok(result)
}

pub(super) fn snapshot(language: &str) -> Result<Value> {
    let result = dictionary(language)?;
    #[cfg(not(webdeck_embedded_frontend))]
    let root = frontend::disk_root("webdeck/translations");
    #[cfg(webdeck_embedded_frontend)]
    let root = std::path::PathBuf::from("webdeck/translations");
    let mut languages = std::collections::BTreeSet::new();
    #[cfg(webdeck_embedded_frontend)]
    languages.extend(
        frontend::EMBEDDED_LANGUAGES
            .iter()
            .map(|name| name.to_string()),
    );
    if let Ok(entries) = std::fs::read_dir(root) {
        languages.extend(
            entries
                .flatten()
                .filter(|entry| {
                    entry.file_type().is_ok_and(|kind| kind.is_file())
                        && entry.path().extension().is_some_and(|ext| ext == "json")
                })
                .filter_map(|entry| {
                    entry
                        .path()
                        .file_stem()
                        .and_then(|name| name.to_str())
                        .filter(|name| valid_catalog_name(name))
                        .map(str::to_owned)
                }),
        );
    } else if languages.is_empty() {
        return Err(Error::execution());
    }
    Ok(json!({"api_version":2,"translations":result,"languages":languages}))
}

#[cfg(test)]
mod translation_tests {
    use super::*;

    #[test]
    fn json_catalog_preserves_unicode_equals_and_escaped_lines() {
        let catalog = decode_catalog(r#"{"greeting":"Español = 日本語","help":"line one\nline two","label":"Editar {label}","legacy":"%count%"}"#).unwrap();
        assert_eq!(catalog["greeting"], "Español = 日本語");
        assert_eq!(catalog["help"], "line one\nline two");
        assert_eq!(catalog["label"], "Editar {label}");
        assert_eq!(catalog["legacy"], "%count%");
    }

    #[test]
    fn catalog_requires_a_flat_string_dictionary() {
        assert!(decode_catalog(r#"{"common.settings":42}"#).is_none());
        assert!(decode_catalog(r#"{"common.settings":{"nested":"value"}}"#).is_none());
        assert!(decode_catalog("not json").is_none());
    }

    #[test]
    fn catalog_names_cannot_traverse_the_filesystem() {
        for name in [
            "../en_US",
            "en_US/../../config",
            "/tmp/secret",
            "",
            "en_US.json",
        ] {
            assert!(!valid_catalog_name(name));
        }
        assert!(valid_catalog_name("en_US"));
        assert!(valid_catalog_name("zh_Hant_TW"));
    }
}
