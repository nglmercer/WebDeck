//! Port of `app/buttons/color_picker/get_mouse_pixel_color.py`.
//!
//! Cursor position via `enigo`, pixel capture via `screenshots`
//! (replacing `pyautogui`+`mss`+`numpy`). The HEX/RGB/HSL formatting and the
//! HSL algorithm are ported exactly.

use enigo::{Enigo, Mouse, Settings};
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

/// Capture one pixel with the `screenshots` crate (X11 / Windows / macOS).
fn capture_pixel_native(x: i32, y: i32) -> Result<[u8; 3], String> {
    let screen = Screen::from_point(x, y).map_err(|e| e.to_string())?;
    let info = screen.display_info;
    let image = screen
        .capture_area(x - info.x, y - info.y, 1, 1)
        .map_err(|e| e.to_string())?;
    let pixel = image.get_pixel(0, 0);
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
    let pixel = capture_pixel_native(x, y).or_else(|e| {
        log().warning(&format!(
            "Color picker: native capture failed ({e}), trying grim"
        ));
        capture_pixel_grim(x, y)
    })?;
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
}
