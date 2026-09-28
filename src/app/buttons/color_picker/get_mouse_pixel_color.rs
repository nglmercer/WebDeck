//! Cursor position via `enigo`, pixel capture per platform.
//!
//! Linux captures without the `screenshots` crate: `grim` on Wayland
//! compositors that ship it (fast subprocess, no D-Bus), the XDG Screenshot
//! portal via `ashpd` (pure-Rust D-Bus), or raw `xcb` GetImage
//! (X11/XWayland). `screenshots` forces `dbus/vendored`, whose libdbus
//! build lacks `HAVE_POLL`, so any D-Bus call with an fd >= 1024 aborts
//! the whole process (`select`/`FD_SET` stack smash) — unfixable from our
//! side while the crate is linked. Other platforms keep `screenshots`
//! (its `dbus` dependency is Linux-only and never builds there).

use enigo::{Enigo, Mouse, Settings};

#[cfg(target_os = "linux")]
use display_info::DisplayInfo;
#[cfg(not(target_os = "linux"))]
use screenshots::Screen;

use crate::app::utils::logger::log;

/// Captured pixel color in the three representations Python returns.
#[derive(Debug, Clone)]
pub struct MousePixelColor {
    pub hex: String,
    pub rgb: String,
    pub hsl: String,
}

/// Format a pixel exactly like Python (`#{:02x}` / `rgb()` / `hsl()`).
pub fn format_pixel_color(red: u8, green: u8, blue: u8) -> MousePixelColor {
    let hex = format!("#{red:02x}{green:02x}{blue:02x}");
    let rgb = format!("rgb({red},{green},{blue})");

    let (r, g, b) = (
        red as f64 / 255.0,
        green as f64 / 255.0,
        blue as f64 / 255.0,
    );
    let cmax = r.max(g).max(b);
    let cmin = r.min(g).min(b);
    let delta = cmax - cmin;

    // NOTE: Python applies `% 6` before `* 60`; for the red-max branch with
    // negative (g − b), Python's float modulo stays positive — mirrored here.
    let hue = if delta == 0.0 {
        0.0
    } else if cmax == r {
        ((g - b) / delta).rem_euclid(6.0)
    } else if cmax == g {
        (b - r) / delta + 2.0
    } else {
        (r - g) / delta + 4.0
    };
    let mut hue = (hue * 60.0).round() as i64;
    if hue < 0 {
        hue += 360;
    }

    let lightness = (cmax + cmin) / 2.0;
    let saturation = if delta == 0.0 {
        0.0
    } else {
        delta / (1.0 - (2.0 * lightness - 1.0).abs())
    };

    let hsl = format!(
        "hsl({hue}, {:.2}%, {:.2}%)",
        saturation * 100.0,
        lightness * 100.0
    );
    MousePixelColor { hex, rgb, hsl }
}

/// Capture one pixel with the `screenshots` crate (Windows / macOS).
#[cfg(not(target_os = "linux"))]
fn capture_pixel_native(x: i32, y: i32) -> Result<[u8; 3], String> {
    let screen = Screen::from_point(x, y).map_err(|e| e.to_string())?;
    let info = screen.display_info;
    let image = screen
        .capture_area(x - info.x, y - info.y, 1, 1)
        .map_err(|e| e.to_string())?;
    let pixel = image.get_pixel(0, 0);
    Ok([pixel[0], pixel[1], pixel[2]])
}

/// Same Wayland heuristic `screenshots` used: trust the session type first,
/// fall back to the Wayland socket variable.
#[cfg(target_os = "linux")]
fn is_wayland() -> bool {
    let session = std::env::var("XDG_SESSION_TYPE").unwrap_or_default();
    if session == "wayland" {
        return true;
    }
    std::env::var("WAYLAND_DISPLAY")
        .unwrap_or_default()
        .to_lowercase()
        .contains("wayland")
}

/// Decode one 8-bit ZPixmap pixel (single-pixel port of the `screenshots`
/// xorg converter, with bounds checks instead of blind indexing).
#[cfg(target_os = "linux")]
fn convert_xpixel_8(bytes: &[u8], lsb_first: bool) -> Result<[u8; 3], String> {
    let raw = *bytes.first().ok_or("xcb: empty 8-bit pixel")?;
    let pixel = if lsb_first {
        raw
    } else {
        raw & 7 << 4 | raw >> 4
    };
    Ok([
        (pixel >> 6) as f32 / 3.0 * 255.0,
        ((pixel >> 2) & 7) as f32 / 7.0 * 255.0,
        (pixel & 3) as f32 / 3.0 * 255.0,
    ]
    .map(|v| v as u8))
}

/// Decode one 16-bit ZPixmap pixel (single-pixel port of `screenshots`).
#[cfg(target_os = "linux")]
fn convert_xpixel_16(bytes: &[u8], lsb_first: bool) -> Result<[u8; 3], String> {
    if bytes.len() < 2 {
        return Err("xcb: truncated 16-bit pixel".to_string());
    }
    let pixel = if lsb_first {
        bytes[0] as u16 | (bytes[1] as u16) << 8
    } else {
        (bytes[0] as u16) << 8 | bytes[1] as u16
    };
    Ok([
        (pixel >> 11) as f32 / 31.0 * 255.0,
        ((pixel >> 5) & 63) as f32 / 63.0 * 255.0,
        (pixel & 31) as f32 / 31.0 * 255.0,
    ]
    .map(|v| v as u8))
}

/// Decode one 24/32-bit ZPixmap pixel (single-pixel port of `screenshots`).
#[cfg(target_os = "linux")]
fn convert_xpixel_24_32(
    bytes: &[u8],
    _bits_per_pixel: u32,
    lsb_first: bool,
) -> Result<[u8; 3], String> {
    // Single-pixel capture: the byte offset is always 0 regardless of
    // stride; the parameter only documents parity with `screenshots`.
    if bytes.len() < 3 {
        return Err("xcb: truncated 24/32-bit pixel".to_string());
    }
    if lsb_first {
        Ok([bytes[2], bytes[1], bytes[0]])
    } else {
        Ok([bytes[0], bytes[1], bytes[2]])
    }
}

/// Capture one pixel over plain X11 (`xcb` GetImage, no D-Bus involved).
#[cfg(target_os = "linux")]
fn capture_pixel_xcb(x: i32, y: i32) -> Result<[u8; 3], String> {
    use xcb::x::{Drawable, GetImage, ImageFormat, ImageOrder};

    let info = DisplayInfo::from_point(x, y).map_err(|e| e.to_string())?;
    let (conn, screen_idx) = xcb::Connection::connect(None).map_err(|e| e.to_string())?;
    let setup = conn.get_setup();
    let screen = setup
        .roots()
        .nth(screen_idx as usize)
        .ok_or("xcb: screen not found")?;
    // Global (root) coords scaled to physical pixels, like `screenshots`.
    let px = ((x as f32) * info.scale_factor) as i32;
    let py = ((y as f32) * info.scale_factor) as i32;
    let cookie = conn.send_request(&GetImage {
        format: ImageFormat::ZPixmap,
        drawable: Drawable::Window(screen.root()),
        x: px.clamp(i32::from(i16::MIN), i32::from(i16::MAX)) as i16,
        y: py.clamp(i32::from(i16::MIN), i32::from(i16::MAX)) as i16,
        width: 1,
        height: 1,
        plane_mask: u32::MAX,
    });
    let reply = conn.wait_for_reply(cookie).map_err(|e| e.to_string())?;
    let bytes = reply.data();
    let depth = reply.depth();
    let pixmap_format = setup
        .pixmap_formats()
        .iter()
        .find(|item| item.depth() == depth)
        .ok_or("xcb: pixmap format not found")?;
    let bits_per_pixel = pixmap_format.bits_per_pixel() as u32;
    let lsb_first = setup.bitmap_format_bit_order() == ImageOrder::LsbFirst;
    match depth {
        8 => convert_xpixel_8(bytes, lsb_first),
        16 => convert_xpixel_16(bytes, lsb_first),
        24 | 32 => convert_xpixel_24_32(bytes, bits_per_pixel, lsb_first),
        other => Err(format!("xcb: unsupported depth {other}")),
    }
}

/// `file://` URI from the portal to a filesystem path (minimal
/// percent-decoding; avoids a new dependency for one call).
#[cfg(target_os = "linux")]
fn file_path_from_uri(uri: &str) -> Result<std::path::PathBuf, String> {
    let path = uri
        .strip_prefix("file://")
        .ok_or_else(|| format!("portal: unexpected screenshot URI {uri:?}"))?;
    let mut out = Vec::with_capacity(path.len());
    let mut bytes = path.as_bytes().iter();
    while let Some(&b) = bytes.next() {
        if b == b'%' {
            let (Some(&hi), Some(&lo)) = (bytes.next(), bytes.next()) else {
                return Err("portal: truncated percent escape".to_string());
            };
            let hex = |c: u8| {
                (c as char)
                    .to_digit(16)
                    .map(|d| d as u8)
                    .ok_or("portal: invalid percent escape".to_string())
            };
            out.push(hex(hi)? << 4 | hex(lo)?);
        } else {
            out.push(b);
        }
    }
    Ok(std::path::PathBuf::from(
        String::from_utf8(out).map_err(|_| "portal: non-UTF8 path".to_string())?,
    ))
}

/// Capture one pixel through the XDG Screenshot portal (`ashpd`, pure-Rust
/// D-Bus — the accurate Wayland path on KDE/GNOME where `grim` is absent).
#[cfg(target_os = "linux")]
fn capture_pixel_portal(x: i32, y: i32) -> Result<[u8; 3], String> {
    let scale = DisplayInfo::from_point(x, y)
        .map(|info| info.scale_factor)
        .unwrap_or(1.0);
    let (px, py) = (
        ((x as f32) * scale).round() as i64,
        ((y as f32) * scale).round() as i64,
    );
    // Callers run on sync `spawn_blocking` threads: bridge the async portal
    // call with a throwaway current-thread runtime (never `block_on` an
    // outer runtime from here).
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| e.to_string())?;
    let png = runtime.block_on(async {
        // NOTE: always hand ashpd a fresh connection. Its internal one is
        // cached in a process-global `OnceLock`, so a second capture would
        // reuse a connection whose I/O tasks belonged to the first call's
        // (now dropped) runtime and hang forever.
        let connection = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            zbus::Connection::session(),
        )
        .await
        .map_err(|_| "portal bus connection timed out".to_string())?
        .map_err(|e| e.to_string())?;
        let request = tokio::time::timeout(std::time::Duration::from_secs(20), async {
            ashpd::desktop::screenshot::Screenshot::request()
                .connection(Some(connection))
                .modal(true)
                .interactive(false)
                .send()
                .await
        })
        .await
        .map_err(|_| "portal screenshot timed out".to_string())?
        .map_err(|e| e.to_string())?;
        let uri = request.response().map_err(|e| e.to_string())?;
        let path = file_path_from_uri(uri.uri().as_str())?;
        let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
        if let Err(e) = std::fs::remove_file(&path) {
            log().debug(&format!("portal: cannot remove {path:?}: {e}"));
        }
        Ok::<Vec<u8>, String>(bytes)
    })?;
    let image = image::load_from_memory(&png)
        .map_err(|e| e.to_string())?
        .to_rgb8();
    if px < 0 || py < 0 || px >= image.width() as i64 || py >= image.height() as i64 {
        return Err(format!(
            "portal: pixel ({px},{py}) outside {}x{} screenshot",
            image.width(),
            image.height()
        ));
    }
    let pixel = image.get_pixel(px as u32, py as u32);
    Ok([pixel[0], pixel[1], pixel[2]])
}

/// Linux fallback for Wayland (where X11 capture fails): grab a 1x1 region
/// with `grim` (wlroots compositors) as raw PPM and parse the single pixel.
#[cfg(target_os = "linux")]
fn capture_pixel_grim(x: i32, y: i32) -> Result<[u8; 3], String> {
    let out = std::process::Command::new("grim")
        .args(["-g", &format!("{x},{y} 1x1"), "-t", "ppm", "-"])
        .output()
        .map_err(|e| format!("grim not available: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "grim failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    parse_p6_1x1(&out.stdout)
}

/// Minimal P6 parser for grim's 1x1 output: magic, dims, maxval, raw RGB.
#[cfg(target_os = "linux")]
fn parse_p6_1x1(buf: &[u8]) -> Result<[u8; 3], String> {
    let mut parts = buf.splitn(4, |b| *b == b'\n');
    let magic = parts.next().unwrap_or_default();
    if magic != b"P6" {
        return Err("grim: unexpected output format".to_string());
    }
    let dims = parts.next().unwrap_or_default();
    let dims = String::from_utf8_lossy(dims);
    let mut it = dims.split_whitespace();
    let (w, h) = (it.next().unwrap_or("0"), it.next().unwrap_or("0"));
    if w != "1" || h != "1" {
        return Err(format!("grim: unexpected geometry {w}x{h}"));
    }
    let _maxval = parts.next();
    let data = parts.next().unwrap_or_default();
    if data.len() < 3 {
        return Err("grim: truncated pixel data".to_string());
    }
    Ok([data[0], data[1], data[2]])
}

/// Port of `get_mouse_pixel_color`.
pub fn get_mouse_pixel_color() -> Result<MousePixelColor, String> {
    let enigo = Enigo::new(&Settings::default()).map_err(|e| e.to_string())?;
    let (x, y) = enigo.location().map_err(|e| e.to_string())?;

    #[cfg(target_os = "linux")]
    let pixel = if is_wayland() {
        // grim is instant where present; the portal is the accurate
        // fallback on KDE/GNOME; xcb sees XWayland clients only, so it
        // stays last (better than failing outright on exotic compositors).
        capture_pixel_grim(x, y).or_else(|grim_err| {
            log().debug(&format!(
                "Color picker: grim unavailable ({grim_err}), trying portal"
            ));
            capture_pixel_portal(x, y).or_else(|portal_err| {
                log().warning(&format!(
                    "Color picker: portal failed ({portal_err}), trying xcb"
                ));
                capture_pixel_xcb(x, y)
            })
        })?
    } else {
        capture_pixel_xcb(x, y)?
    };
    #[cfg(not(target_os = "linux"))]
    let pixel = capture_pixel_native(x, y)?;

    log().debug(&format!(
        "Mouse pixel at ({x}, {y}): rgb({},{},{})",
        pixel[0], pixel[1], pixel[2]
    ));
    Ok(format_pixel_color(pixel[0], pixel[1], pixel[2]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_os = "linux")]
    #[test]
    fn parses_grim_p6_1x1() {
        let ppm = b"P6\n1 1\n255\n\x12\x34\x56";
        assert_eq!(parse_p6_1x1(ppm), Ok([0x12, 0x34, 0x56]));
        assert!(parse_p6_1x1(b"P5\n1 1\n255\n\x00").is_err());
        assert!(parse_p6_1x1(b"P6\n2 2\n255\n\x00\x00\x00").is_err());
    }

    #[test]
    fn formats_primary_colors() {
        let red = format_pixel_color(255, 0, 0);
        assert_eq!(red.hex, "#ff0000");
        assert_eq!(red.rgb, "rgb(255,0,0)");
        assert_eq!(red.hsl, "hsl(0, 100.00%, 50.00%)");

        let gray = format_pixel_color(128, 128, 128);
        assert_eq!(gray.hex, "#808080");
        assert_eq!(gray.hsl, "hsl(0, 0.00%, 50.20%)");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn decodes_x11_pixel_formats() {
        // 32-bit LSB (BGRA order on little-endian X11).
        assert_eq!(
            convert_xpixel_24_32(&[0x56, 0x34, 0x12, 0x00], 32, true),
            Ok([0x12, 0x34, 0x56])
        );
        assert_eq!(
            convert_xpixel_24_32(&[0x12, 0x34, 0x56], 24, false),
            Ok([0x12, 0x34, 0x56])
        );
        // 16-bit 5-6-5 red.
        assert_eq!(convert_xpixel_16(&[0x00, 0xF8], true), Ok([255, 0, 0]));
        // 8-bit pseudo-color extremes map into range without panicking.
        assert!(convert_xpixel_8(&[0x00], true).is_ok());
        assert!(convert_xpixel_8(&[0xFF], false).is_ok());
        // Truncated replies are errors, never panics.
        assert!(convert_xpixel_24_32(&[], 32, true).is_err());
        assert!(convert_xpixel_16(&[0x00], true).is_err());
        assert!(convert_xpixel_8(&[], true).is_err());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn decodes_portal_file_uris() {
        assert_eq!(
            file_path_from_uri("file:///tmp/shot.png").unwrap(),
            std::path::PathBuf::from("/tmp/shot.png")
        );
        assert_eq!(
            file_path_from_uri("file:///tmp/my%20shot%25x.png").unwrap(),
            std::path::PathBuf::from("/tmp/my shot%x.png")
        );
        assert!(file_path_from_uri("https://example.com/x.png").is_err());
        assert!(file_path_from_uri("file:///tmp/bad%.png").is_err());
        assert!(file_path_from_uri("file:///tmp/bad%zz.png").is_err());
    }
}
