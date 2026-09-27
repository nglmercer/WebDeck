//! Port of `app/buttons/window/focus_window.py`.
//!
//! `FindWindow` + `SW_RESTORE` + `SetForegroundWindow` via the `windows`
//! crate. [`foreground`] brings an already-known hwnd forward (what
//! `/firstplan` needs); [`bring_window_to_front`] looks the title up first.

use crate::app::utils::logger::log;

/// Port of `bring_window_to_front`.
pub fn bring_window_to_front(window_title: &str) -> Result<(), String> {
    #[cfg(windows)]
    {
        use windows::Win32::Foundation::HWND;
        use windows::Win32::UI::WindowsAndMessaging::{
            FindWindowW, SetForegroundWindow, ShowWindow, SW_RESTORE,
        };
        use windows::core::{HSTRING, PCWSTR};

        let title = HSTRING::from(window_title);
        let hwnd = unsafe { FindWindowW(PCWSTR::null(), PCWSTR(title.as_ptr())) }
            .unwrap_or(HWND(std::ptr::null_mut()));
        if hwnd.0.is_null() {
            log().error(&format!("Window with title '{window_title}' not found"));
            return Err(format!("Window with title '{window_title}' not found"));
        }
        unsafe {
            let _ = ShowWindow(hwnd, SW_RESTORE);
            let _ = SetForegroundWindow(hwnd);
        }
        Ok(())
    }
    #[cfg(not(windows))]
    {
        log().warning(&format!(
            "bring_window_to_front({window_title:?}): only supported on Windows"
        ));
        Err(format!("Window with title '{window_title}' not found"))
    }
}

/// Bring an hwnd (from [`super::find_window::get_window_by_name`]) to the
/// foreground — port of the `SetForegroundWindow(hwnd)` in `/firstplan`.
pub fn foreground(hwnd: isize) {
    #[cfg(windows)]
    {
        use windows::Win32::Foundation::HWND;
        use windows::Win32::UI::WindowsAndMessaging::SetForegroundWindow;
        unsafe {
            let _ = SetForegroundWindow(HWND(hwnd as *mut std::ffi::c_void));
        }
    }
    #[cfg(not(windows))]
    {
        let _ = hwnd;
    }
}
