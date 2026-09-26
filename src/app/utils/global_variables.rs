//! Port of `app/utils/global_variables.py`.
//!
//! Python uses a plain module-level dict; Rust uses a `Mutex`-guarded static
//! map. Values are `serde_json::Value` (the dynamic values stored here —
//! `config`, plugin metadata — are all JSON-shaped).
//!
//! Plugin *callables* cannot live in a JSON map; those live in the
//! [`crate::app::utils::plugins::load_plugins`] registry instead.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use serde_json::Value;

static VARS: OnceLock<Mutex<HashMap<String, Value>>> = OnceLock::new();

fn map() -> &'static Mutex<HashMap<String, Value>> {
    VARS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Port of `set_global_variable`.
pub fn set_global_variable(variable_name: &str, value: Value) {
    if let Ok(mut vars) = map().lock() {
        vars.insert(variable_name.to_string(), value);
    }
}

/// Port of `get_global_variable` (returns `None` for missing keys, like `.get`).
pub fn get_global_variable(variable_name: &str) -> Option<Value> {
    map().lock().ok()?.get(variable_name).cloned()
}

/// Port of `get_global_variables` — fetch several variables at once.
///
/// Python takes/returns tuples; Rust uses fixed-size arrays, which keeps the
/// same call shape: `get_global_variables(["obs_host", "obs_port"])`.
pub fn get_global_variables<const N: usize>(variable_names: [&str; N]) -> [Option<Value>; N] {
    let vars = map().lock();
    variable_names.map(|name| {
        vars.as_ref()
            .ok()
            .and_then(|vars| vars.get(name).cloned())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn set_and_get_roundtrip() {
        set_global_variable("test_key_rs", json!({"a": 1}));
        assert_eq!(get_global_variable("test_key_rs"), Some(json!({"a": 1})));
        assert_eq!(get_global_variable("missing_key_rs"), None);
    }

    #[test]
    fn get_many() {
        set_global_variable("test_many_a", json!(1));
        let [a, missing] = get_global_variables(["test_many_a", "test_many_missing"]);
        assert_eq!(a, Some(json!(1)));
        assert_eq!(missing, None);
    }
}
