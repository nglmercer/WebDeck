//! Port of `app/buttons/window/close_window.py`.
//!
//! `win32gui.FindWindow` + `PostMessage(WM_CLOSE)` via the `windows` crate.

use crate::app::utils::logger::log;

/// Port of `close_window` — closes by exact window title.
pub fn close_window(window_title: &str) -> Result<(), String> {
    #[cfg(windows)]
    {
        use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
        use windows::Win32::UI::WindowsAndMessaging::{FindWindowW, PostMessageW, WM_CLOSE};
        use windows::core::{HSTRING, PCWSTR};

        let title = HSTRING::from(window_title);
        let hwnd = unsafe { FindWindowW(PCWSTR::null(), PCWSTR(title.as_ptr())) }
            .unwrap_or(HWND(std::ptr::null_mut()));
        if hwnd.0.is_null() {
            log().warning(&format!(
                "Window with title '{window_title}' not found or already closed"
            ));
            return Err(format!(
                "Window with title '{window_title}' not found or already closed"
            ));
        }
        unsafe {
            PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0)).map_err(|e| e.to_string())?;
        }
        Ok(())
    }
    #[cfg(not(windows))]
    {
        log().warning(&format!(
            "close_window({window_title:?}): only supported on Windows"
        ));
        Err(format!(
            "Window with title '{window_title}' not found or already closed"
        ))
    }
}
