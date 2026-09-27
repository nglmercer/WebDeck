//! Tray menu state (extracted from `tray.rs`).
//!
//! Desired menu state, config reloads, and the language/server-state
//! request plumbing. `TrayIcon` is `!Send`, so cross-thread updates go
//! through this plain-data state instead of a shared icon handle.

use std::sync::atomic::AtomicBool;
use std::sync::{Mutex, OnceLock};

use super::ServerState;
use crate::app::utils::get_local_ip::get_local_ip;
use crate::app::utils::languages::set_default_language;
use crate::app::utils::settings::get_config::{get_config, get_port, save_config};

/// Desired tray menu state, polled by the tray thread.
///
/// `TrayIcon` is `!Send`, so the icon stays owned by the thread running
/// [`super::create_tray_icon`] and cross-thread updates (language / server
/// state) go through this plain-data state instead of a shared icon handle.
pub(crate) struct DesiredTrayState {
    pub(crate) language: String,
    pub(crate) status: ServerState,
    pub(crate) dirty: bool,
}

static TRAY_STATE: OnceLock<Mutex<DesiredTrayState>> = OnceLock::new();
pub(crate) static TRAY_RUNNING: AtomicBool = AtomicBool::new(false);

pub(crate) fn tray_state() -> &'static Mutex<DesiredTrayState> {
    TRAY_STATE.get_or_init(|| {
        Mutex::new(DesiredTrayState {
            language: String::new(),
            status: ServerState::Loading,
            dirty: true,
        })
    })
}

/// Request a tray menu rebuild in `language` (applied by the tray thread).
fn request_tray_language(language: &str) {
    if let Ok(mut state) = tray_state().lock() {
        state.language = language.to_string();
        state.dirty = true;
    }
}

/// Request a tray menu rebuild with `status` (applied by the tray thread).
fn request_tray_status(status: ServerState) {
    if let Ok(mut state) = tray_state().lock() {
        state.status = status;
        state.dirty = true;
    }
}

/// Port of `reload_config`: (port, dark_theme, language, integrated_browser).
pub fn reload_config() -> (u16, bool, String, bool) {
    let config = get_config(true, false);
    let port = get_port();
    let dark_theme = config
        .get("front")
        .and_then(|f| f.get("dark_theme"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let settings = config.get("settings");
    let language = settings
        .and_then(|s| s.get("language"))
        .and_then(|v| v.as_str())
        .unwrap_or("system")
        .to_string();
    let integrated = settings
        .and_then(|s| s.get("open_settings_in_integrated_browser"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    (port, dark_theme, language, integrated)
}

/// Best-effort local IP (Python computes it once at import).
pub(crate) fn local_ip() -> String {
    get_local_ip().unwrap_or_else(|_| "127.0.0.1".to_string())
}

/// Port of `change_tray_language`: regenerate the menu in the new language
/// (server state resets to the default online, exactly as in Python).
pub fn change_tray_language(new_lang: &str) {
    request_tray_language(new_lang);
    // Python also resets the server state to the default online here.
    request_tray_status(ServerState::Running);
}

/// Port of `update_language`: default language + tray menu + saved config.
pub fn update_language(new_lang: &str) {
    set_default_language(new_lang);
    change_tray_language(new_lang);

    let mut config = get_config(true, false);
    if let Some(lang_slot) = config
        .get_mut("settings")
        .and_then(|s| s.get_mut("language"))
    {
        *lang_slot = serde_json::Value::from(new_lang.to_string());
    }
    save_config(&config);
}

/// Port of `change_server_state`.
pub fn change_server_state(new_state: ServerState) {
    request_tray_status(new_state);
}
