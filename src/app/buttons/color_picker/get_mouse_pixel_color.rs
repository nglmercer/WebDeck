//! Port of `app/buttons/color_picker/get_mouse_pixel_color.py`.
//!
//! TODO(port): cursor position + screen capture via `enigo` (cursor) and
//! `screenshots`/`xcap` (pixels), then RGB→HSL conversion.

use crate::app::utils::logger::log;

/// Captured pixel color in the three representations Python returns.
#[derive(Debug, Clone)]
pub struct MousePixelColor {
    pub hex: String,
    pub rgb: String,
    pub hsl: String,
}

/// Port of `get_mouse_pixel_color` — stub.
pub fn get_mouse_pixel_color() -> Result<MousePixelColor, String> {
    log().warning("get_mouse_pixel_color: screen capture backend not ported yet");
    Err("screen capture backend not ported yet".to_string())
}
