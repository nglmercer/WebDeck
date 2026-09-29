//! Port of `app/tray.py`.
//!
//! Backend mapping (behavior kept 1:1 with the Python build):
//! - `pystray` tray icon + menu -> [`tray_icon`] (+ `muda` menu items).
//! - `tkinter` QR window -> `webdeck-qr` child process (`minifb`, no webview).
//! - `tkinter` port-prompt window -> `tao` + `wry` window.
//! - `pywebview` integrated config window -> `tao` + `wry` maximized window.
//! - `webbrowser.open` -> [`crate::app::buttons::system::openfile::openfile`].
//! - `PIL.Image.open("*.ico")` -> [`image`] crate decode.
//! - `qrcode` -> [`qrcode`] crate (EC level L, auto version, 290px).
//!
//! Python runs the tray on Windows only; the Rust port enables the same tray
//! on Linux too (StatusNotifier icon + native webviews). Other platforms get
//! inert stubs.
//! Deviations forced by the backend (documented, no behavior lost):
//! - `show_qrcode` re-entry is guarded (a second call is a no-op) instead of
//!   lifting the existing window, since tao gives no cross-thread lift API.
//! - Language radio items are check items; the menu is rebuilt on change.
//! - The integrated config window is maximized at creation instead of via a
//!   post-hoc `ShowWindow(SW_MAXIMIZE)` call.
//!
//! Split: [`state`] (menu state + config), [`menu`] (menu build/dispatch),
//! [`windows`] (QR/config/port windows); this file keeps the icon loop,
//! `ServerState`, port validation, and the non-desktop stubs.

#[cfg(any(windows, target_os = "linux"))]
mod menu;
#[cfg(any(windows, target_os = "linux"))]
mod state;
#[cfg(any(windows, target_os = "linux"))]
mod windows;

#[cfg(any(windows, target_os = "linux"))]
pub use state::{change_server_state, change_tray_language, reload_config, update_language};
#[cfg(any(windows, target_os = "linux"))]
pub use windows::{change_port_prompt, open_config, show_qrcode};

#[cfg(any(windows, target_os = "linux"))]
use self::menu::{dispatch_menu, generate_menu};
#[cfg(any(windows, target_os = "linux"))]
use self::state::{tray_state, TRAY_RUNNING};
#[cfg(any(windows, target_os = "linux"))]
use self::windows::load_icon_rgba;
#[cfg(any(windows, target_os = "linux"))]
use crate::app::utils::logger::log;
#[cfg(any(windows, target_os = "linux"))]
use std::sync::atomic::Ordering;
#[cfg(any(windows, target_os = "linux"))]
use tray_icon::menu::MenuEvent;
#[cfg(any(windows, target_os = "linux"))]
use tray_icon::{TrayIconBuilder, TrayIconEvent};

/// Server reachability state shown in the tray menu.
///
/// Numeric mapping mirrors Python (`server_status=0/1/2`):
/// Loading = 0, Running = 1, Stopped = 2.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ServerState {
    Loading,
    Running,
    Stopped,
}

/// Save-time port verdict: all digits, 1-65535, not empty, not the current
/// port. (Python additionally allows `""` mid-typing; our page only
/// validates on save, which is the same observable gate.)
pub fn validate_port_value(value: &str, current: u16) -> bool {
    let trimmed = value.trim();
    if trimmed.is_empty() || !trimmed.bytes().all(|b| b.is_ascii_digit()) {
        return false;
    }
    match trimmed.parse::<u32>() {
        Ok(n) => (1..=65535).contains(&n) && n != current as u32,
        Err(_) => false,
    }
}

/// Port of `generate_tray_icon` + `create_tray_icon`: build the icon
/// (tooltip `WebDeck`, or `WebDeck DEV` in debug builds like Python's
/// non-frozen branch) and service menu events. Blocks forever.
#[cfg(any(windows, target_os = "linux"))]
pub fn create_tray_icon() {
    if TRAY_RUNNING.swap(true, Ordering::SeqCst) {
        return;
    }
    let (rgba, w, h) = match load_icon_rgba("static/icons/icon.ico", 32) {
        Some(icon) => icon,
        None => {
            log().warning("cannot load static/icons/icon.ico; tray disabled");
            return;
        }
    };
    let icon = match tray_icon::Icon::from_rgba(rgba, w, h) {
        Ok(icon) => icon,
        Err(e) => {
            log().warning(&format!("icon decode failed: {e}"));
            return;
        }
    };
    // Seed the desired state (an early `change_server_state` from the
    // server thread may already have set a non-loading status).
    let (_, _, config_language, _) = reload_config();
    let initial = if let Ok(mut state) = tray_state().lock() {
        if state.language.is_empty() {
            state.language = config_language.clone();
        }
        state.dirty = false;
        (state.language.clone(), state.status)
    } else {
        (config_language, ServerState::Loading)
    };
    let menu = generate_menu(&initial.0, initial.1);
    let tooltip = if cfg!(debug_assertions) {
        "WebDeck DEV"
    } else {
        "WebDeck"
    };
    let tray = match TrayIconBuilder::new()
        .with_tooltip(tooltip)
        .with_icon(icon)
        .with_menu(Box::new(menu))
        .build()
    {
        Ok(t) => t,
        Err(e) => {
            log().warning(&format!("tray build failed: {e}"));
            return;
        }
    };

    // Double-click opens the same QR window as the default menu entry.
    std::thread::spawn(|| {
        for event in TrayIconEvent::receiver() {
            if matches!(event, TrayIconEvent::DoubleClick { .. }) {
                show_qrcode();
            }
        }
    });

    // Menu-event service loop (blocks like pystray's `icon.run()`),
    // polling for requested menu rebuilds along the way.
    let menu_rx = MenuEvent::receiver();
    loop {
        let pending = tray_state().lock().ok().and_then(|mut state| {
            if state.dirty {
                state.dirty = false;
                Some((state.language.clone(), state.status))
            } else {
                None
            }
        });
        if let Some((language, status)) = pending {
            tray.set_menu(Some(Box::new(generate_menu(&language, status))));
        }
        if let Ok(event) = menu_rx.try_recv() {
            dispatch_menu(event.id.0.as_str());
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}

/// Non-desktop stub (macOS and other Unixes without the GUI stack).
#[cfg(not(any(windows, target_os = "linux")))]
pub fn create_tray_icon() {
    eprintln!("[tray] system tray is only supported on Windows and Linux");
}

/// Non-Windows stub.
#[cfg(not(any(windows, target_os = "linux")))]
pub fn show_qrcode() {}

/// Non-Windows stub.
#[cfg(not(any(windows, target_os = "linux")))]
pub fn open_config() {}

/// Non-Windows stub.
#[cfg(not(any(windows, target_os = "linux")))]
pub fn reload_config() -> (u16, bool, String, bool) {
    (5000, false, "system".to_string(), false)
}

/// Non-Windows stub.
#[cfg(not(any(windows, target_os = "linux")))]
pub fn update_language(_language: &str) {}

/// Non-Windows stub.
#[cfg(not(any(windows, target_os = "linux")))]
pub fn change_port_prompt() {}

/// Non-Windows stub.
#[cfg(not(any(windows, target_os = "linux")))]
pub fn change_tray_language(_language: &str) {}

/// Non-Windows stub.
#[cfg(not(any(windows, target_os = "linux")))]
pub fn change_server_state(_server_state: ServerState) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn port_validation_matches_python_save_gate() {
        assert!(validate_port_value("8080", 8000));
        assert!(validate_port_value("1", 8000));
        assert!(validate_port_value("65535", 8000));
        assert!(validate_port_value(" 8080 ", 8000));
        assert!(!validate_port_value("0", 8000));
        assert!(!validate_port_value("65536", 8000));
        assert!(!validate_port_value("abc", 8000));
        assert!(!validate_port_value("80.5", 8000));
        assert!(!validate_port_value("", 8000));
        assert!(!validate_port_value("8000", 8000));
        assert!(!validate_port_value(" 8000 ", 8000));
    }

    #[test]
    fn server_state_numeric_mapping_matches_python() {
        assert_eq!(ServerState::Loading as u8, 0);
        assert_eq!(ServerState::Running as u8, 1);
        assert_eq!(ServerState::Stopped as u8, 2);
    }
}
