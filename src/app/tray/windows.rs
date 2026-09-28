//! Tray windows: config, port prompt (extracted from `tray.rs`).
//!
//! The tao/wry window runner plus the two windows built on it. The QR
//! popup is a separate `webdeck-qr` child process instead (see
//! [`show_qrcode`]): one light `minifb` window, no webview.

use std::sync::atomic::{AtomicBool, Ordering};

use super::state::{local_ip, reload_config};
use crate::app::buttons::system::openfile::openfile;
use crate::app::utils::languages::text;
use crate::app::utils::logger::log;
use crate::app::utils::qr;
use crate::app::utils::restart::restart_program;
use crate::app::utils::settings::get_config::{get_config, get_port, save_config};

/// Python keeps the tkinter window in a global; a bool guard is the
/// cross-thread equivalent (tao windows cannot be lifted from here).
static QR_OPEN: AtomicBool = AtomicBool::new(false);

/// Stdin of the running QR viewer, when one is open: each line is a frame
/// command for the child's protocol (`<png-path>` swaps the frame,
/// `QUIT` closes). Held apart from the `Child` so frame pushes never
/// block on the spawner thread's `wait`.
static QR_STDIN: std::sync::OnceLock<std::sync::Mutex<Option<std::process::ChildStdin>>> =
    std::sync::OnceLock::new();

fn qr_stdin_slot() -> &'static std::sync::Mutex<Option<std::process::ChildStdin>> {
    QR_STDIN.get_or_init(|| std::sync::Mutex::new(None))
}

/// Load an app `.ico` file as RGBA pixels resized to `size`x`size`.
pub(crate) fn load_icon_rgba(path: &str, size: u32) -> Option<(Vec<u8>, u32, u32)> {
    let img = image::open(path).ok()?;
    let img = img.resize_exact(size, size, image::imageops::FilterType::Lanczos3);
    let rgba = img.to_rgba8();
    let (w, h) = (rgba.width(), rgba.height());
    Some((rgba.into_raw(), w, h))
}

/// Port of `show_qrcode`: QR image + URL label, not resizable,
/// Escape/Return/Space close.
/// Rendered by a detached `webdeck-qr` child process (light `minifb`
/// window, no webview); a second call while one is open stays a no-op.
pub fn show_qrcode() {
    if QR_OPEN.swap(true, Ordering::SeqCst) {
        return;
    }
    std::thread::spawn(|| {
        let (_, _, language, _) = reload_config();
        let url = format!("http://{}:{}/", local_ip(), get_port());
        if let Err(e) = spawn_qr_viewer(&url, &text(Some("qr_code"), Some(&language))) {
            log().warning(&format!("qr viewer failed: {e}"));
        }
        *qr_stdin_slot().lock().unwrap_or_else(|e| e.into_inner()) = None;
        QR_OPEN.store(false, Ordering::SeqCst);
    });
}

/// Write the QR PNG handoff file and run the viewer to completion.
/// Blocks until the viewer exits (runs on the spawner thread).
fn spawn_qr_viewer(url: &str, title: &str) -> Result<(), String> {
    let png =
        qr::generate_qr_code_png(url).ok_or_else(|| format!("cannot encode {url:?} as QR"))?;
    let path = qr::qr_png_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("cannot create temp dir: {e}"))?;
    }
    std::fs::write(&path, png).map_err(|e| format!("cannot write {}: {e}", path.display()))?;

    let mut cmd = std::process::Command::new(qr::resolve_viewer_binary());
    cmd.arg(format!("--png={}", path.display()));
    cmd.arg(format!("--title={title}"));
    cmd.arg(format!("--label={url}"));
    cmd.stdin(std::process::Stdio::piped());
    #[cfg(windows)]
    {
        // Keep the child from flashing a console window.
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("cannot spawn webdeck-qr: {e}"))?;
    *qr_stdin_slot().lock().unwrap_or_else(|e| e.into_inner()) = child.stdin.take();
    child
        .wait()
        .map_err(|e| format!("webdeck-qr failed: {e}"))?;
    Ok(())
}

/// Push a new frame (PNG path) to the open QR viewer, if any.
/// Returns false when no viewer is running (call [`show_qrcode`] first).
#[allow(dead_code)]
pub fn push_qr_frame(png_path: &std::path::Path) -> bool {
    use std::io::Write;
    let mut slot = qr_stdin_slot().lock().unwrap_or_else(|e| e.into_inner());
    match slot.as_mut() {
        Some(stdin) => writeln!(stdin, "{}", png_path.display())
            .and_then(|_| stdin.flush())
            .is_ok(),
        None => false,
    }
}

/// Port of `open_config`: integrated maximized window or external browser.
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
                    Ok(n) if super::validate_port_value(msg.trim(), current) => n,
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
#[derive(Clone, Copy)]
enum CloseKeys {
    /// Escape only (config + port windows handle Return themselves).
    /// (The QR window moved to the `webdeck-qr` child, which closes on
    /// Escape/Return/Space itself.)
    EscapeOnly,
}

/// Run a tao/wry window with the given builder. Blocks until closed.
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
