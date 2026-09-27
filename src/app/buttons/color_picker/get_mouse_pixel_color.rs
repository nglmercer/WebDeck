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

    let hsl = format!("hsl({hue}, {:.2}%, {:.2}%)", saturation * 100.0, lightness * 100.0);
    MousePixelColor { hex, rgb, hsl }
}

/// Port of `get_mouse_pixel_color`.
pub fn get_mouse_pixel_color() -> Result<MousePixelColor, String> {
    let enigo = Enigo::new(&Settings::default()).map_err(|e| e.to_string())?;
    let (x, y) = enigo.location().map_err(|e| e.to_string())?;

    let screen = Screen::from_point(x, y).map_err(|e| e.to_string())?;
    let info = screen.display_info;
    let image = screen
        .capture_area(x - info.x, y - info.y, 1, 1)
        .map_err(|e| e.to_string())?;
    let pixel = image.get_pixel(0, 0);

    log().debug(&format!("Mouse pixel at ({x}, {y}): rgb{}", &format!("({},{},{})", pixel[0], pixel[1], pixel[2])));
    Ok(format_pixel_color(pixel[0], pixel[1], pixel[2]))
}

#[cfg(test)]
mod tests {
    use super::*;

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
