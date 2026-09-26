//! Port of `app/buttons/window/focus_window.py`.
//!
//! TODO(port): `FindWindow` + `SW_RESTORE` + `SetForegroundWindow` via the
//! `windows` crate.

use crate::app::utils::logger::log;

/// Port of `bring_window_to_front` — stub.
pub fn bring_window_to_front(window_title: &str) {
    log().warning(&format!(
        "bring_window_to_front({window_title:?}): windows backend not ported yet"
    ));
}
