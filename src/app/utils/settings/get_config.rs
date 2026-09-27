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

use crate::app::utils::{
    args::get_args, settings::check_config_update::check_config_update, working_dir,
};

/// Config directory: `.config` under the base dir, or `$WEBDECK_CONFIG_DIR`
/// when set (test seam so unit tests never touch the real config).
pub fn config_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("WEBDECK_CONFIG_DIR") {
        PathBuf::from(dir)
    } else {
        working_dir::get_base_dir().join(".config")
    }
}

/// Port of the module-level `config_path` in `app/utils/settings/get_config.py`.
pub fn get_config_path() -> PathBuf {
    config_dir().join("config.json")
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
///
/// Writes atomically (temp file + rename) so concurrent readers never see a
/// torn config — the server reads config from many tasks at once.
pub fn save_config(config: &Value) {
    let content = serde_json::to_string_pretty(config).expect("Cannot serialize config");
    write_config_atomically(&get_config_path(), &content);
}

/// Atomic file write used by both `save_config` copies.
pub fn write_config_atomically(path: &std::path::Path, content: &str) {
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, content).expect("Cannot write config temp file");
    std::fs::rename(&tmp, path).expect("Cannot replace config file");
}

/// Test-only helpers: serialize disk-config tests and point them at a temp
/// config dir so the real `.config/config.json` is never touched.
#[cfg(test)]
pub mod test_support {
    use std::sync::{Mutex, OnceLock};

    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    /// Hold the returned guard for the whole test. Points
    /// `$WEBDECK_CONFIG_DIR` at a shared temp dir (same value every time,
    /// so concurrent readers are safe; the lock serializes writers).
    pub fn config_guard() -> std::sync::MutexGuard<'static, ()> {
        let guard = LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let dir = std::env::temp_dir().join("webdeck-test-config");
        std::fs::create_dir_all(&dir).unwrap();
        std::env::set_var("WEBDECK_CONFIG_DIR", &dir);
        guard
    }

    /// Overwrite the temp config with `value` (call while holding the guard).
    pub fn seed_config(value: &serde_json::Value) {
        let dir = std::env::temp_dir().join("webdeck-test-config");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("config.json"),
            serde_json::to_string(value).unwrap(),
        )
        .unwrap();
    }
}
