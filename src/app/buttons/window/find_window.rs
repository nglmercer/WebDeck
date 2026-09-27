//! Port of `app/buttons/window/find_window.py`.
//!
//! Window walking mirrors Python exactly: `FindWindow(None, None)` then the
//! `GW_HWNDNEXT` chain, fuzzy-matching titles with `.exe` stripped.

#[cfg(not(windows))]
use crate::app::utils::logger::log;

/// Opaque window handle (port of the `hwnd` ints Python threads around).
pub type WindowRef = isize;

/// Pure title predicate (the match inside `find_window_with_name`).
pub fn title_matches(window_title: &str, name: &str) -> bool {
    let normalize = |s: &str| s.to_lowercase().replace(".exe", "");
    normalize(window_title).contains(&normalize(name))
}

/// Read a window's title (port of `win32gui.GetWindowText`).
#[cfg(windows)]
fn window_text(hwnd: isize) -> String {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::GetWindowTextW;
    let mut buffer = [0u16; 512];
    let length =
        unsafe { GetWindowTextW(HWND(hwnd as *mut std::ffi::c_void), &mut buffer) } as usize;
    String::from_utf16_lossy(&buffer[..length.min(buffer.len())])
}

/// Port of `find_window_with_name` — returns the hwnd on match, else `None`.
pub fn find_window_with_name(hwnd: isize, name: &str) -> Option<isize> {
    #[cfg(windows)]
    {
        if title_matches(&window_text(hwnd), name) {
            Some(hwnd)
        } else {
            None
        }
    }
    #[cfg(not(windows))]
    {
        let _ = (hwnd, name);
        None
    }
}

/// Port of `get_window_by_name` — first fuzzy match in Z-order, walking the
/// `GW_HWNDNEXT` chain from `FindWindow(None, None)`.
pub fn get_window_by_name(name: &str) -> Result<isize, String> {
    #[cfg(windows)]
    {
        use windows::Win32::Foundation::HWND;
        use windows::Win32::UI::WindowsAndMessaging::{FindWindowW, GetWindow, GW_HWNDNEXT};
        use windows::core::PCWSTR;

        let mut hwnd = unsafe { FindWindowW(PCWSTR::null(), PCWSTR::null()) }
            .unwrap_or(HWND(std::ptr::null_mut()));
        while !hwnd.0.is_null() {
            let handle = hwnd.0 as isize;
            if find_window_with_name(handle, name).is_some() {
                return Ok(handle);
            }
            hwnd = unsafe { GetWindow(hwnd, GW_HWNDNEXT) }.unwrap_or(HWND(std::ptr::null_mut()));
        }
        Err(format!("Window '{name}' not found"))
    }
    #[cfg(not(windows))]
    {
        log().warning(&format!("get_window_by_name({name:?}): only supported on Windows"));
        Err(format!("Window '{name}' not found"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_case_insensitively() {
        assert!(title_matches("Untitled - Notepad", "notepad"));
        assert!(title_matches("code.exe", "Code"));
        assert!(!title_matches("Explorer", "notepad"));
    }
}
