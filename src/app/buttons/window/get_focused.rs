//! Port of `app/buttons/window/get_focused.py`.
//!
//! TODO(port): `GetForegroundWindow` + `GetWindowText` via the `windows` crate.

use crate::app::utils::logger::log;

/// Port of `get_focused_window` — stub.
pub fn get_focused_window() -> Result<String, String> {
    log().debug("No window has focus (windows backend not ported yet)");
    Err("No window has focus".to_string())
}
