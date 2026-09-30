//! Port of `app/buttons/window/close_window.py`.
//!
//! `win32gui.FindWindow` + `PostMessage(WM_CLOSE)` via the `windows` crate.

use crate::app::utils::logger::log;

/// Port of `close_window` — closes by exact window title.
pub fn close_window(window_title: &str) -> Result<(), String> {
    #[cfg(windows)]
    {
        use windows::core::{HSTRING, PCWSTR};
        use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
        use windows::Win32::UI::WindowsAndMessaging::{FindWindowW, PostMessageW, WM_CLOSE};

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
    // Exact title match (port of FindWindow's equality), then close.
    #[cfg(target_os = "linux")]
    {
        match super::find_window::list_windows() {
            Ok(windows) => match windows.into_iter().find(|(_, title)| title == window_title) {
                Some((id, _)) => super::find_window::close_window_by_id(id),
                None => {
                    log().warning(&format!(
                        "Window with title '{window_title}' not found or already closed"
                    ));
                    Err(format!(
                        "Window with title '{window_title}' not found or already closed"
                    ))
                }
            },
            Err(e) => {
                log().warning(&format!("close_window({window_title:?}): {e}"));
                Err(format!(
                    "Window with title '{window_title}' not found or already closed"
                ))
            }
        }
    }
    #[cfg(not(any(windows, target_os = "linux")))]
    {
        log().warning(&format!(
            "close_window({window_title:?}): only supported on Windows and Linux"
        ));
        Err(format!(
            "Window with title '{window_title}' not found or already closed"
        ))
    }
}
