//! `webdeck-qr` — lightweight standalone QR viewer.
//!
//! Shows one QR bitmap plus an `EXIT` button in a native `minifb` window:
//! no webview, no UI toolkit, no GPU. Spawned by the tray parent (see
//! `tray::windows::show_qrcode`), but any process can reuse it:
//!
//! ```sh
//! webdeck-qr --png temp/qr.png --label "http://192.168.1.2:5000/"
//! webdeck-qr --text "http://192.168.1.2:5000/" --size 310x360
//! ```
//!
//! Frame protocol (for future reuse): while running, the viewer reads
//! stdin lines on a helper thread — a line with an existing PNG path swaps
//! the displayed frame, `QUIT` closes the window. A null/closed stdin
//! simply yields no frames.

use std::io::BufRead;
use std::path::PathBuf;
use std::sync::mpsc;

use clap::Parser;
use minifb::{Key, KeyRepeat, MouseButton, MouseMode, Window, WindowOptions};
use webdeck::app::utils::qr;

/// Lightweight QR viewer: one QR bitmap plus an EXIT button.
#[derive(Parser, Debug)]
#[command(name = "webdeck-qr")]
struct QrArgs {
    /// PNG image file to display (mutually exclusive with --text)
    #[arg(long)]
    png: Option<PathBuf>,

    /// Raw payload to encode as QR (mutually exclusive with --png)
    #[arg(long)]
    text: Option<String>,

    /// Extra label appended to the window title (e.g. the encoded URL)
    #[arg(long)]
    label: Option<String>,

    /// Window title
    #[arg(long, default_value = "WebDeck QR code")]
    title: String,

    /// Window size as WxH
    #[arg(long, default_value = "310x360")]
    size: String,
}

fn load_gray(args: &QrArgs) -> Result<image::GrayImage, String> {
    match (&args.png, &args.text) {
        (Some(path), None) => image::open(path)
            .map(|img| img.to_luma8())
            .map_err(|e| format!("cannot open --png {}: {e}", path.display())),
        (None, Some(text)) => qr::render_qr_luma(text, qr::QR_DISPLAY_SIZE)
            .ok_or_else(|| "cannot encode --text as QR".to_string()),
        (Some(_), Some(_)) => Err("--png and --text are mutually exclusive".to_string()),
        (None, None) => Err("one of --png or --text is required".to_string()),
    }
}

fn window_title(args: &QrArgs) -> String {
    match &args.label {
        Some(label) => format!("{} - {label}", args.title),
        None => args.title.clone(),
    }
}

/// Stdin frame source: PNG path per line swaps the frame, `QUIT` exits.
/// Runs on its own thread so a missing stdin never blocks the UI loop.
fn spawn_frame_source() -> mpsc::Receiver<String> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        for line in std::io::stdin().lock().lines().map_while(Result::ok) {
            if tx.send(line).is_err() {
                break;
            }
        }
    });
    rx
}

fn main() {
    let args = QrArgs::parse();

    let (w, h) = match qr::parse_size(&args.size) {
        Some(size) => size,
        None => {
            eprintln!(
                "webdeck-qr: invalid --size {:?}, want WxH like 310x360",
                args.size
            );
            std::process::exit(2);
        }
    };
    let mut gray = match load_gray(&args) {
        Ok(img) => img,
        Err(e) => {
            eprintln!("webdeck-qr: {e}");
            std::process::exit(1);
        }
    };
    let mut frame = qr::compose_frame(&gray, w, h);

    let mut window = match Window::new(
        &window_title(&args),
        w as usize,
        h as usize,
        WindowOptions {
            resize: false,
            ..WindowOptions::default()
        },
    ) {
        Ok(window) => window,
        Err(e) => {
            eprintln!("webdeck-qr: cannot open window: {e}");
            std::process::exit(1);
        }
    };
    window.set_target_fps(30);

    let frames = spawn_frame_source();
    let button = qr::button_rect(w, h);
    let mut mouse_was_down = false;
    let mut should_close = false;

    while window.is_open() && !should_close {
        if window.is_key_pressed(Key::Escape, KeyRepeat::No)
            || window.is_key_pressed(Key::Enter, KeyRepeat::No)
            || window.is_key_pressed(Key::Space, KeyRepeat::No)
        {
            break;
        }
        let mouse_down = window.get_mouse_down(MouseButton::Left);
        if mouse_down && !mouse_was_down {
            if let Some((x, y)) = window.get_mouse_pos(MouseMode::Discard) {
                if qr::hit_test(x, y, button) {
                    break;
                }
            }
        }
        mouse_was_down = mouse_down;

        while let Ok(line) = frames.try_recv() {
            let line = line.trim();
            if line.eq_ignore_ascii_case("QUIT") {
                should_close = true;
            } else if !line.is_empty() {
                match image::open(line) {
                    Ok(img) => {
                        gray = img.to_luma8();
                        frame = qr::compose_frame(&gray, w, h);
                    }
                    Err(e) => eprintln!("webdeck-qr: frame reload failed for {line:?}: {e}"),
                }
            }
        }

        if window
            .update_with_buffer(&frame, w as usize, h as usize)
            .is_err()
        {
            break;
        }
    }
}
