//! Canonical portable key vocabulary. Mapping support is distinct from desktop permission.
use crate::{
    contracts::ErrorCode,
    domain::{Error, Result},
};
use serde::Deserialize;
use std::{
    collections::{BTreeMap, HashSet},
    sync::OnceLock,
};
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyEntry {
    pub id: String,
    pub label: String,
    pub category: String,
    pub aliases: Vec<String>,
    pub kind: String,
    pub platform_support: BTreeMap<String, String>,
}
#[derive(Deserialize)]
struct Catalog {
    keys: Vec<KeyEntry>,
}
pub fn catalog() -> &'static [KeyEntry] {
    static CATALOG: OnceLock<Catalog> = OnceLock::new();
    &CATALOG
        .get_or_init(|| {
            serde_json::from_str(include_str!("../contracts/keys.json"))
                .expect("validated key catalog")
        })
        .keys
}
pub fn lookup(key: &str) -> Option<&'static KeyEntry> {
    catalog().iter().find(|entry| {
        entry.id.eq_ignore_ascii_case(key)
            || entry
                .aliases
                .iter()
                .any(|alias| alias.eq_ignore_ascii_case(key))
    })
}
pub fn normalize(key: &str) -> Result<String> {
    if let Some(entry) = lookup(key) {
        return Ok(entry.id.clone());
    }
    // Preserve legacy single Unicode keys. Arbitrary text belongs to `write`.
    if key.chars().count() == 1 && !key.chars().any(char::is_control) {
        return Ok(key.into());
    }
    Err(Error::new(
        ErrorCode::InvalidInput,
        "Unknown keyboard key; choose a key from the catalog",
    ))
}
pub fn chord(keys: &[String]) -> Result<Vec<String>> {
    if keys.is_empty() || keys.len() > 16 {
        return Err(Error::invalid());
    }
    let normalized = keys
        .iter()
        .map(|k| normalize(k))
        .collect::<Result<Vec<_>>>()?;
    let mut seen = HashSet::new();
    for key in &normalized {
        if !seen.insert(key) {
            return Err(Error::new(
                ErrorCode::InvalidInput,
                "Duplicate key or modifier alias in chord",
            ));
        }
        if lookup(key).is_some_and(|entry| {
            entry
                .platform_support
                .values()
                .all(|support| support == "unsupported")
        }) {
            return Err(Error::new(
                ErrorCode::UnsupportedPlatform,
                "Key is unavailable on this backend; Fn is normally handled by keyboard firmware",
            ));
        }
    }
    Ok(normalized)
}
/// Static implementation support; never implies live desktop authorization.
pub fn platform_support(key: &str, platform: &str) -> &'static str {
    lookup(key)
        .and_then(|entry| entry.platform_support.get(platform))
        .map(String::as_str)
        .unwrap_or("unsupported")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn aliases_and_function_keys() {
        assert_eq!(
            chord(&["Control".into(), "SHIFT".into(), "F12".into()]).unwrap(),
            ["ctrl", "shift", "f12"]
        );
        for n in 1..=24 {
            assert_eq!(normalize(&format!("F{n}")).unwrap(), format!("f{n}"));
        }
        for keys in [vec![], vec!["ctrl", "control"], vec!["fn"], vec!["unknown"]] {
            assert!(chord(&keys.into_iter().map(String::from).collect::<Vec<_>>()).is_err());
        }
    }
    #[test]
    fn catalog_is_unambiguous() {
        let mut aliases = HashSet::new();
        for entry in catalog() {
            for alias in std::iter::once(&entry.id).chain(&entry.aliases) {
                assert!(aliases.insert(alias.to_lowercase()), "duplicate {alias}");
            }
        }
    }
}
