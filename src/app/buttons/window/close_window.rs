//! Port of `app/buttons/window/close_window.py`.
//!
//! TODO(port): `FindWindow` + `WM_CLOSE` via the `windows` crate.

use crate::app::utils::logger::log;

/// Opaque window reference (port of the `hwnd`/title the Python code threads
/// around; becomes `HWND` once the `windows` backend lands).
pub type WindowRef = String;

/// Port of `close_window`.
pub fn close_window(window_title: &str) -> Result<(), String> {
    log().warning(&format!(
        "close_window({window_title:?}): windows backend not ported yet"
    ));
    Err(format!(
        "Window with title '{window_title}' not found or already closed"
    ))
}
