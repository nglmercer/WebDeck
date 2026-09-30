//! Port of `app/buttons/window/get_focused.py`.
//!
//! `GetForegroundWindow` + `GetWindowText` via the `windows` crate. Like
//! Python, returns the focused window's *title* (used for `taskkill /im`
//! by `/superAltF4`).

use crate::app::utils::logger::log;

/// Port of `get_focused_window`.
pub fn get_focused_window() -> Result<String, String> {
    #[cfg(windows)]
    {
        use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowTextW};

        let hwnd = unsafe { GetForegroundWindow() };
        if hwnd.0.is_null() {
            log().debug("No window has focus");
            return Err("No window has focus".to_string());
        }
        let mut buffer = [0u16; 512];
        let length = unsafe { GetWindowTextW(hwnd, &mut buffer) } as usize;
        let title = String::from_utf16_lossy(&buffer[..length.min(buffer.len())]).to_string();
        log().debug(&format!("Focused window: {title}"));
        Ok(title)
    }
    #[cfg(target_os = "linux")]
    {
        use super::find_window::{run_output, tool_present};
        if !tool_present("xdotool") {
            log().debug("No window has focus (needs xdotool on X11/XWayland)");
            return Err("No window has focus".to_string());
        }
        let id = run_output("xdotool", &["getwindowfocus"])
            .map(|s| s.trim().to_string())
            .map_err(|_| "No window has focus".to_string())?;
        let title = run_output("xdotool", &["getwindowname", &id])
            .map(|s| s.trim().to_string())
            .map_err(|_| "No window has focus".to_string())?;
        log().debug(&format!("Focused window: {title}"));
        Ok(title)
    }
    #[cfg(not(any(windows, target_os = "linux")))]
    {
        log().debug("No window has focus (only supported on Windows and Linux)");
        Err("No window has focus".to_string())
    }
}
