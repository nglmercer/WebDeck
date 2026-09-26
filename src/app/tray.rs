//! Port of `app/tray.py`.
//!
//! [`reload_config`] and the config-write half of [`update_language`] are
//! ported 1:1 (pure config logic). The `pystray` icon/menu, tkinter windows
//! (QR code, port prompt), and `pywebview` are TODO — planned via
//! `tray-icon` + `wry` + `rfd` (see `docs/MIGRATION_RUST.md`).

use std::sync::{Mutex, OnceLock};

use crate::app::utils::{
    languages::set_default_language,
    logger::log,
    settings::get_config::{get_config, get_port},
};

/// Tray state — port of the module-level `icon`/`language`/… globals.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerState {
    Loading = 0,
    Online = 1,
    Offline = 2,
}

static TRAY_LANGUAGE: OnceLock<Mutex<String>> = OnceLock::new();
static SERVER_STATE: OnceLock<Mutex<ServerState>> = OnceLock::new();

fn tray_language() -> &'static Mutex<String> {
    TRAY_LANGUAGE.get_or_init(|| Mutex::new("en_US".to_string()))
}

fn server_state() -> &'static Mutex<ServerState> {
    SERVER_STATE.get_or_init(|| Mutex::new(ServerState::Loading))
}

/// Port of `reload_config` — returns
/// `(port, dark_theme, language, open_in_integrated_browser)`.
pub fn reload_config() -> (u16, bool, String, bool) {
    let config = get_config(true, false);
    let port = config
        .get("url")
        .and_then(|u| u.get("port"))
        .and_then(|p| p.as_u64())
        .map(|p| p as u16)
        .unwrap_or_else(get_port);
    let dark_theme = config
        .get("front")
        .and_then(|f| f.get("dark_theme"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let language = config
        .get("settings")
        .and_then(|s| s.get("language"))
        .and_then(|v| v.as_str())
        .unwrap_or("en_US")
        .to_string();
    let open_in_integrated_browser = config
        .get("settings")
        .and_then(|s| s.get("open_settings_in_integrated_browser"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    (port, dark_theme, language, open_in_integrated_browser)
}

/// Port of `open_config` — stub (wry webview / browser launch TODO).
pub fn open_config() {
    log().warning("open_config: tray webview/browser launch not ported yet (wry planned)");
}

/// Port of `generate_qr_code` — stub (`qrcode` crate TODO).
pub fn generate_qr_code(_dark_theme: bool) -> Result<Vec<u8>, String> {
    Err("generate_qr_code: qrcode backend not ported yet".to_string())
}

/// Port of `show_qrcode` — stub (native window TODO).
pub fn show_qrcode() {
    log().warning("show_qrcode: QR window not ported yet");
}

/// Port of `generate_menu` — stub (`tray-icon` menu TODO).
pub fn generate_menu(language: &str, server_status: ServerState) {
    log().info(&format!(
        "Server status updated: {} (tray menu for language {language} not ported yet)",
        server_status as u8
    ));
}

/// Port of `generate_tray_icon` — stub (`tray-icon` TODO).
pub fn generate_tray_icon() {
    log().warning("generate_tray_icon: tray-icon backend not ported yet");
}

/// Port of `change_port_prompt` — stub (`rfd` dialog TODO).
pub fn change_port_prompt() {
    log().warning("change_port_prompt: port dialog not ported yet (rfd planned)");
}

/// Port of `change_tray_language`.
pub fn change_tray_language(new_lang: &str) {
    if let Ok(mut lang) = tray_language().lock() {
        *lang = new_lang.to_string();
    }
    let state = server_state().lock().map(|s| *s).unwrap_or(ServerState::Loading);
    generate_menu(new_lang, state);
}

/// Port of `update_language` — the config write is real; the menu refresh
/// delegates to the (stubbed) tray menu.
pub fn update_language(new_lang: &str) {
    set_default_language(new_lang);
    change_tray_language(new_lang);

    let mut config = get_config(false, false);
    if let Some(settings) = config.get_mut("settings").and_then(|s| s.as_object_mut()) {
        settings.insert(
            "language".to_string(),
            serde_json::Value::String(new_lang.to_string()),
        );
    }
    crate::app::utils::settings::save_config::save_config(config);
}

/// Port of `change_server_state`.
pub fn change_server_state(new_state: u8) {
    let state = match new_state {
        0 => ServerState::Loading,
        1 => ServerState::Online,
        _ => ServerState::Offline,
    };
    if let Ok(mut slot) = server_state().lock() {
        *slot = state;
    }
    let language = tray_language().lock().map(|l| l.clone()).unwrap_or_default();
    generate_menu(&language, state);
}

/// Port of `create_tray_icon`.
///
/// Python blocks in `icon.run()`; until the `tray-icon` backend lands this
/// parks the thread (never returns) so `main` keeps the same blocking shape.
pub fn create_tray_icon() {
    generate_tray_icon();
    log().warning("create_tray_icon: parking thread until the tray-icon backend lands");
    loop {
        std::thread::park();
    }
}
