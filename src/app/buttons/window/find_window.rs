//! Port of `app/buttons/window/find_window.py`.
//!
//! TODO(port): window enumeration via the `windows` crate
//! (`EnumWindows`/`GetWindowText`).

use crate::app::buttons::window::close_window::WindowRef;
use crate::app::utils::logger::log;

/// Port of `find_window_with_name`.
///
/// Pure predicate over a window title; kept for parity (the enumeration
/// itself is what needs the `windows` backend).
pub fn find_window_with_name(window_title: &str, name: &str) -> bool {
    let normalize = |s: &str| s.to_lowercase().replace(".exe", "");
    normalize(window_title).contains(&normalize(name))
}

/// Port of `get_window_by_name` — stub.
pub fn get_window_by_name(name: &str) -> Result<WindowRef, String> {
    log().warning(&format!(
        "get_window_by_name({name:?}): windows backend not ported yet"
    ));
    Err(format!("Window '{name}' not found"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_case_insensitively() {
        assert!(find_window_with_name("Untitled - Notepad", "notepad"));
        assert!(find_window_with_name("code.exe", "Code"));
        assert!(!find_window_with_name("Explorer", "notepad"));
    }
}
