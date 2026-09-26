//! Port of `app/utils/settings/get_config.py`.
//!
//! Config-source-of-truth notes (same as Python):
//! - `.config/config.json` is created from `webdeck/config_default.json` on
//!   first run (`ensure_config_exists`, including the legacy `config.json`
//!   move).
//! - `--port` overrides `url.port` (`get_port`).
//! - Like Python (where config errors are fatal/uncaught), IO/JSON failures
//!   panic with a clear message instead of threading `Result` through every
//!   caller.

use std::path::PathBuf;

use serde_json::Value;

use crate::app::utils::{args::get_args, settings::check_config_update::check_config_update, working_dir};

/// Port of the module-level `config_path` in `app/utils/settings/get_config.py`.
pub fn get_config_path() -> PathBuf {
    working_dir::get_base_dir().join(".config").join("config.json")
}

fn default_config_path() -> PathBuf {
    working_dir::get_base_dir()
        .join("webdeck")
        .join("config_default.json")
}

/// Port of `ensure_config_exists`.
pub fn ensure_config_exists() {
    let config_path = get_config_path();
    if std::path::Path::new("config.json").exists() {
        let _ = std::fs::rename("config.json", &config_path);
    } else if !config_path.exists() {
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent).expect("Cannot create .config directory");
        }
        let default_content = std::fs::read_to_string(default_config_path())
            .expect("Cannot read webdeck/config_default.json");
        std::fs::write(&config_path, default_content).expect("Cannot write .config/config.json");
    }
    if std::path::Path::new("config.json").exists() {
        let _ = std::fs::remove_file("config.json");
    }
}

/// Port of `get_config`.
pub fn get_config(check_updates: bool, save_updated_config: bool) -> Value {
    ensure_config_exists();

    let content =
        std::fs::read_to_string(get_config_path()).expect("Cannot read .config/config.json");
    let mut config: Value =
        serde_json::from_str(&content).expect("Cannot parse .config/config.json");

    if check_updates || save_updated_config {
        config = check_config_update(config);
        if save_updated_config {
            save_config(&config);
        }
    }

    config
}

/// Port of `get_port`.
pub fn get_port() -> u16 {
    if let Some(port) = get_args().port {
        return port;
    }
    get_config(false, false)
        .get("url")
        .and_then(|u| u.get("port"))
        .and_then(|p| p.as_u64())
        .map(|p| p as u16)
        .unwrap_or(5000)
}

/// Port of `save_config` (the copy living in `get_config.py`; the canonical
/// one with global-variable sync lives in `settings::save_config`).
pub fn save_config(config: &Value) {
    let content = serde_json::to_string_pretty(config).expect("Cannot serialize config");
    std::fs::write(get_config_path(), content).expect("Cannot write .config/config.json");
}
