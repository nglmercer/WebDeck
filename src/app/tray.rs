//! Port of `app/tray.py`.
//!
//! Backend mapping (behavior kept 1:1 with the Python build):
//! - `pystray` tray icon + menu -> [`tray_icon`] (+ `muda` menu items).
//! - `tkinter` QR / port-prompt windows -> `tao` + `wry` windows.
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

#[cfg(any(windows, target_os = "linux"))]
use crate::app::buttons::system::openfile::openfile;
#[cfg(any(windows, target_os = "linux"))]
use crate::app::utils::exit::exit_program;
#[cfg(any(windows, target_os = "linux"))]
use crate::app::utils::firewall::fix_firewall_permission;
#[cfg(any(windows, target_os = "linux"))]
use crate::app::utils::get_local_ip::get_local_ip;
#[cfg(any(windows, target_os = "linux"))]
use crate::app::utils::languages::{get_language, get_languages_info, set_default_language, text};
#[cfg(any(windows, target_os = "linux"))]
use crate::app::utils::logger::log;
#[cfg(any(windows, target_os = "linux"))]
use crate::app::utils::restart::restart_program;
#[cfg(any(windows, target_os = "linux"))]
use crate::app::utils::settings::get_config::{get_config, get_port, save_config};
#[cfg(any(windows, target_os = "linux"))]
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(any(windows, target_os = "linux"))]
use std::sync::{Mutex, OnceLock};
#[cfg(any(windows, target_os = "linux"))]
use tray_icon::menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};
#[cfg(any(windows, target_os = "linux"))]
use tray_icon::{TrayIconBuilder, TrayIconEvent};

/// Desired tray menu state, polled by the tray thread.
///
/// `TrayIcon` is `!Send`, so the icon stays owned by the thread running
/// [`create_tray_icon`] and cross-thread updates (language / server state)
/// go through this plain-data state instead of a shared icon handle.
#[cfg(any(windows, target_os = "linux"))]
struct DesiredTrayState {
    language: String,
    status: ServerState,
    dirty: bool,
}

#[cfg(any(windows, target_os = "linux"))]
static TRAY_STATE: OnceLock<Mutex<DesiredTrayState>> = OnceLock::new();
#[cfg(any(windows, target_os = "linux"))]
static TRAY_RUNNING: AtomicBool = AtomicBool::new(false);

#[cfg(any(windows, target_os = "linux"))]
fn tray_state() -> &'static Mutex<DesiredTrayState> {
    TRAY_STATE.get_or_init(|| {
        Mutex::new(DesiredTrayState {
            language: String::new(),
            status: ServerState::Loading,
            dirty: true,
        })
    })
}

/// Request a tray menu rebuild in `language` (applied by the tray thread).
#[cfg(any(windows, target_os = "linux"))]
fn request_tray_language(language: &str) {
    if let Ok(mut state) = tray_state().lock() {
        state.language = language.to_string();
        state.dirty = true;
    }
}

/// Request a tray menu rebuild with `status` (applied by the tray thread).
#[cfg(any(windows, target_os = "linux"))]
fn request_tray_status(status: ServerState) {
    if let Ok(mut state) = tray_state().lock() {
        state.status = status;
        state.dirty = true;
    }
}
/// Python keeps the tkinter window in a global; a bool guard is the
/// cross-thread equivalent (tao windows cannot be lifted from here).
#[cfg(any(windows, target_os = "linux"))]
static QR_OPEN: AtomicBool = AtomicBool::new(false);

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

/// Port of `reload_config`: (port, dark_theme, language, integrated_browser).
#[cfg(any(windows, target_os = "linux"))]
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
#[cfg(any(windows, target_os = "linux"))]
fn local_ip() -> String {
    get_local_ip().unwrap_or_else(|_| "127.0.0.1".to_string())
}

/// Port of `generate_qr_code`: EC-L QR PNG bytes for `url`, ~290px,
/// black-on-white (Python's `show_qrcode` always uses `dark_theme=False`).
#[cfg(any(windows, target_os = "linux"))]
fn generate_qr_code_png(url: &str) -> Option<Vec<u8>> {
    let code = qrcode::QrCode::with_error_correction_level(url, qrcode::EcLevel::L).ok()?;
    let image = code
        .render::<image::Luma<u8>>()
        .max_dimensions(290, 290)
        .build();
    let mut png = Vec::new();
    image
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .ok()?;
    Some(png)
}

/// Port of `generate_menu`. Item ids are stable dispatch keys.
#[cfg(any(windows, target_os = "linux"))]
fn generate_menu(language: &str, server_status: ServerState) -> Menu {
    log().info(&format!("Server status updated: {}", server_status as u8));

    let lang = Some(language);
    let status_text = match server_status {
        ServerState::Loading => text(Some("server_loading"), lang),
        ServerState::Running => text(Some("server_online"), lang),
        ServerState::Stopped => text(Some("server_offline"), lang),
    };

    let menu = Menu::new();
    let item_qr = MenuItem::with_id("qr", text(Some("qr_code"), lang), true, None);

    let submenu = Submenu::new(text(Some("options"), lang), true);
    let item_open_config =
        MenuItem::with_id("open_config", text(Some("open_config"), lang), true, None);

    // Language submenu: `native_name (code)` unless identical, misc
    // languages after a separator, current language checked.
    let lang_menu = Submenu::new(text(Some("language"), lang), true);
    let resolved = get_language(Some(language));
    let mut infos = get_languages_info();
    infos.sort_by(|a, b| a.misc.cmp(&b.misc));
    let mut with_separator = false;
    for info in &infos {
        if info.misc && !with_separator {
            let _ = lang_menu.append(&PredefinedMenuItem::separator());
            with_separator = true;
        }
        let label = if info.native_name != info.code {
            format!("{} ({})", info.native_name, info.code)
        } else {
            info.code.clone()
        };
        let check = CheckMenuItem::with_id(
            format!("lang:{}", info.code),
            label,
            true,
            info.code == resolved,
            None,
        );
        let _ = lang_menu.append(&check);
    }

    let item_restart = MenuItem::with_id(
        "restart",
        text(Some("restart_application"), lang),
        true,
        None,
    );
    let item_edit_port = MenuItem::with_id("edit_port", text(Some("edit_port"), lang), true, None);
    let item_fix_firewall =
        MenuItem::with_id("fix_firewall", text(Some("fix_firewall"), lang), true, None);
    let _ = submenu.append_items(&[
        &item_open_config,
        &lang_menu,
        &item_restart,
        &item_edit_port,
        &item_fix_firewall,
    ]);

    let item_server = MenuItem::with_id(
        "server_status",
        format!("{} {status_text}", text(Some("server_status"), lang)),
        true,
        None,
    );
    let item_issue =
        MenuItem::with_id("report_issue", text(Some("report_issue"), lang), true, None);
    let item_exit = MenuItem::with_id("exit", text(Some("exit"), lang), true, None);

    let _ = menu.append_items(&[&item_qr, &submenu, &item_server]);
    let _ = menu.append(&PredefinedMenuItem::separator());
    let _ = menu.append_items(&[&item_issue, &item_exit]);
    menu
}

/// Dispatch a tray menu activation by item id.
#[cfg(any(windows, target_os = "linux"))]
fn dispatch_menu(id: &str) {
    match id {
        "qr" => show_qrcode(),
        "open_config" | "server_status" => open_config(),
        "restart" => restart_program(),
        "edit_port" => change_port_prompt(),
        "fix_firewall" => fix_firewall_permission(),
        "report_issue" => {
            openfile("https://github.com/Lenochxd/WebDeck/issues");
        }
        "exit" => exit_program(true, false),
        lang if lang.starts_with("lang:") => {
            update_language(lang.trim_start_matches("lang:"));
        }
        _ => {}
    }
}

/// Load an app `.ico` file as RGBA pixels resized to `size`x`size`.
#[cfg(any(windows, target_os = "linux"))]
fn load_icon_rgba(path: &str, size: u32) -> Option<(Vec<u8>, u32, u32)> {
    let img = image::open(path).ok()?;
    let img = img.resize_exact(size, size, image::imageops::FilterType::Lanczos3);
    let rgba = img.to_rgba8();
    let (w, h) = (rgba.width(), rgba.height());
    Some((rgba.into_raw(), w, h))
}

/// Port of `show_qrcode`: QR image + URL label, not resizable,
/// Escape/Return/Space close.
#[cfg(any(windows, target_os = "linux"))]
pub fn show_qrcode() {
    if QR_OPEN.swap(true, Ordering::SeqCst) {
        return;
    }
    std::thread::spawn(|| {
        let (_, _, language, _) = reload_config();
        let url = format!("http://{}:{}/", local_ip(), get_port());
        let png = generate_qr_code_png(&url).unwrap_or_default();
        let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &png);
        let html = format!(
            "<!doctype html><html><body style=\"background:#fff;color:#000;font-family:sans-serif;text-align:center;margin:0;padding:8px\">\
             <img src=\"data:image/png;base64,{b64}\" width=\"290\" height=\"290\"/>\
             <p style=\"margin:6px 0 0;font-size:13px;font-family:Helvetica,sans-serif\">{url}</p></body></html>"
        );
        run_window(
            &text(Some("qr_code"), Some(&language)),
            310,
            360,
            false,
            false,
            CloseKeys::Activate,
            move |builder| builder.with_html(html),
        );
        QR_OPEN.store(false, Ordering::SeqCst);
    });
}

/// Port of `open_config`: integrated maximized window or external browser.
#[cfg(any(windows, target_os = "linux"))]
pub fn open_config() {
    std::thread::spawn(|| {
        let (_, _, _, integrated) = reload_config();
        let config_url = format!("http://{}:{}?config=show", local_ip(), get_port());
        if integrated {
            // Python maximizes post-hoc via win32gui when the title contains
            // "webdeck"; maximizing at creation is the same end state.
            run_window(
                "WebDeck Config",
                900,
                700,
                true,
                true,
                CloseKeys::EscapeOnly,
                move |builder| builder.with_url(config_url),
            );
        } else {
            openfile(&config_url);
        }
    });
}

/// Port of `change_port_prompt`: label + entry + randomize + save.
/// Return saves, Escape closes, window icon is `icon_black.ico`.
#[cfg(any(windows, target_os = "linux"))]
pub fn change_port_prompt() {
    std::thread::spawn(|| {
        let (_, _, language, _) = reload_config();
        let current = get_port();
        let lang = Some(language.as_str());
        let html = format!(
            "<!doctype html><html><body style=\"background:#f0f0f0;color:#000;font-family:sans-serif;margin:0;padding:10px\">\
             <label style=\"font-size:14px\">{enter}</label><br/>\
             <input id=\"p\" value=\"{current}\" style=\"font-size:14px;width:96%;margin:5px 0;padding:4px\"/><br/>\
             <button id=\"rnd\" style=\"font-size:14px;width:100%;margin:3px 0;padding:4px\">{random}</button>\
             <button id=\"ok\" style=\"font-size:14px;width:100%;margin:3px 0;padding:4px\">{save}</button>\
             <script>const cur='{current}';\
             const inp=document.getElementById('p');\
             document.getElementById('rnd').onclick=()=>{{inp.value=Math.floor(Math.random()*64512)+1024}};\
             function save(){{const v=inp.value.trim();\
             if(!/^\\d+$/.test(v))return;\
             const n=Number(v);\
             if(v===''||v===cur||n<1||n>65535)return;\
             window.ipc.postMessage(v)}}\
             document.getElementById('ok').onclick=save;\
             inp.addEventListener('keydown',e=>{{if(e.key==='Enter')save()}});\
             inp.focus();</script></body></html>",
            enter = text(Some("enter_new_port"), lang),
            random = text(Some("randomize"), lang),
            save = text(Some("save"), lang),
        );
        run_window_with_ipc(
            &text(Some("change_server_port"), lang),
            300,
            160,
            "static/icons/icon_black.ico",
            html,
            move |msg: String| {
                let new_port: u16 = match msg.trim().parse() {
                    Ok(n) if validate_port_value(msg.trim(), current) => n,
                    _ => return,
                };
                let mut config = get_config(true, false);
                if let Some(port_slot) = config.get_mut("url").and_then(|u| u.get_mut("port")) {
                    *port_slot = serde_json::Value::from(new_port as u64);
                }
                save_config(&config);
                restart_program();
            },
        );
    });
}

/// Which keys close a tao window (Python binds differ per window).
#[cfg(any(windows, target_os = "linux"))]
#[derive(Clone, Copy)]
enum CloseKeys {
    /// Escape/Return/Space (QR window).
    Activate,
    /// Escape only (config + port windows handle Return themselves).
    EscapeOnly,
}

/// Run a tao/wry window with the given builder. Blocks until closed.
#[cfg(any(windows, target_os = "linux"))]
#[allow(clippy::too_many_arguments)]
fn run_window<F>(
    title: &str,
    width: u32,
    height: u32,
    resizable: bool,
    maximized: bool,
    close_keys: CloseKeys,
    build: F,
) where
    F: FnOnce(wry::WebViewBuilder) -> wry::WebViewBuilder,
{
    run_window_inner(
        title,
        width,
        height,
        resizable,
        maximized,
        close_keys,
        "static/icons/icon.ico",
        build,
        None,
    );
}

/// Same as [`run_window`] plus a JS `window.ipc.postMessage` handler.
#[cfg(any(windows, target_os = "linux"))]
fn run_window_with_ipc(
    title: &str,
    width: u32,
    height: u32,
    icon_path: &'static str,
    html: String,
    on_message: impl Fn(String) + Send + 'static,
) {
    run_window_inner(
        title,
        width,
        height,
        false,
        false,
        CloseKeys::EscapeOnly,
        icon_path,
        move |builder| builder.with_html(html),
        Some(Box::new(on_message) as Box<dyn Fn(String) + Send>),
    );
}

/// Core tao/wry window runner with optional IPC callback.
#[cfg(any(windows, target_os = "linux"))]
#[allow(clippy::too_many_arguments)]
fn run_window_inner<F>(
    title: &str,
    width: u32,
    height: u32,
    resizable: bool,
    maximized: bool,
    close_keys: CloseKeys,
    icon_path: &str,
    build: F,
    on_message: Option<Box<dyn Fn(String) + Send>>,
) where
    F: FnOnce(wry::WebViewBuilder) -> wry::WebViewBuilder,
{
    use tao::event::{ElementState, Event, WindowEvent};
    use tao::event_loop::{ControlFlow, EventLoop};
    use tao::keyboard::KeyCode;
    use tao::window::WindowBuilder;

    let event_loop = EventLoop::new();
    let icon = load_icon_rgba(icon_path, 32)
        .and_then(|(rgba, w, h)| tao::window::Icon::from_rgba(rgba, w, h).ok());
    let mut wb = WindowBuilder::new()
        .with_title(title)
        .with_inner_size(tao::dpi::LogicalSize::new(width as f64, height as f64))
        .with_resizable(resizable)
        .with_maximized(maximized);
    if let Some(icon) = icon {
        wb = wb.with_window_icon(Some(icon));
    }
    let window = match wb.build(&event_loop) {
        Ok(w) => w,
        Err(e) => {
            log().warning(&format!("window build failed: {e}"));
            return;
        }
    };
    let builder = build(wry::WebViewBuilder::new());
    let builder = match on_message {
        Some(cb) => builder.with_ipc_handler(move |req: wry::http::Request<String>| {
            cb(req.into_body());
        }),
        None => builder,
    };
    if let Err(e) = builder.build(&window) {
        log().warning(&format!("webview build failed: {e}"));
        return;
    }
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                WindowEvent::KeyboardInput { event, .. } => {
                    if event.state != ElementState::Pressed {
                        return;
                    }
                    let close = match close_keys {
                        CloseKeys::EscapeOnly => event.physical_key == KeyCode::Escape,
                        CloseKeys::Activate => matches!(
                            event.physical_key,
                            KeyCode::Escape
                                | KeyCode::Enter
                                | KeyCode::NumpadEnter
                                | KeyCode::Space
                        ),
                    };
                    if close {
                        *control_flow = ControlFlow::Exit;
                    }
                }
                _ => {}
            },
            _ => {}
        }
    });
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
        Ok(n) => n >= 1 && n <= 65535 && n != current as u32,
        Err(_) => false,
    }
}

/// Port of `change_tray_language`: regenerate the menu in the new language
/// (server state resets to the default online, exactly as in Python).
#[cfg(any(windows, target_os = "linux"))]
pub fn change_tray_language(new_lang: &str) {
    request_tray_language(new_lang);
    // Python also resets the server state to the default online here.
    request_tray_status(ServerState::Running);
}

/// Port of `update_language`: default language + tray menu + saved config.
#[cfg(any(windows, target_os = "linux"))]
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
#[cfg(any(windows, target_os = "linux"))]
pub fn change_server_state(new_state: ServerState) {
    request_tray_status(new_state);
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

    #[cfg(any(windows, target_os = "linux"))]
    #[test]
    fn menu_builds_with_all_states() {
        crate::app::utils::languages::init(
            "webdeck/translations",
            Some("webdeck/translations/misc"),
            "en_US",
        );
        for state in [
            ServerState::Running,
            ServerState::Stopped,
            ServerState::Loading,
        ] {
            let _ = generate_menu("en", state);
        }
    }
}
