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
    #[cfg(target_os = "linux")]
    {
        match window_title(hwnd) {
            Some(title) if title_matches(&title, name) => Some(hwnd),
            _ => None,
        }
    }
    #[cfg(not(any(windows, target_os = "linux")))]
    {
        let _ = (hwnd, name);
        None
    }
}

/// Linux window-listing backend (X11/XWayland only — Wayland compositors
/// expose no window enumeration by design).
///
/// Prefers a single `wmctrl -l` call; falls back to `xdotool` enumeration
/// (`search` + `getwindowname` per id). Returns `(id, title)` pairs with
/// the raw X window id (decimal), matching on titles happens in Rust with
/// [`title_matches`] so fuzzy semantics equal Python's `GW_HWNDNEXT` walk.
#[cfg(target_os = "linux")]
pub(crate) fn list_windows() -> Result<Vec<(isize, String)>, String> {
    if tool_present("wmctrl") {
        let out = run_output("wmctrl", &["-l"])?;
        return Ok(parse_wmctrl_list(&out));
    }
    if tool_present("xdotool") {
        let out = run_output("xdotool", &["search", "--onlyvisible", "--name", "."])?;
        let mut windows = Vec::new();
        for id_text in out.lines().map(str::trim).filter(|l| !l.is_empty()) {
            let Ok(id) = id_text.parse::<isize>() else {
                continue;
            };
            if let Ok(title) = run_output("xdotool", &["getwindowname", id_text]) {
                windows.push((id, title.trim().to_string()));
            }
        }
        return Ok(windows);
    }
    Err("no wmctrl or xdotool on PATH (window listing needs X11/XWayland)".to_string())
}

/// Read one window's title (port of `win32gui.GetWindowText`).
#[cfg(target_os = "linux")]
pub(crate) fn window_title(id: isize) -> Option<String> {
    list_windows()
        .ok()?
        .into_iter()
        .find(|(found, _)| *found == id)
        .map(|(_, title)| title)
}

/// Activate a window by id (port of `SW_RESTORE` + `SetForegroundWindow`).
#[cfg(target_os = "linux")]
pub(crate) fn activate_window(id: isize) -> Result<(), String> {
    if tool_present("xdotool") {
        return run_output("xdotool", &["windowactivate", &id.to_string()]).map(|_| ());
    }
    if tool_present("wmctrl") {
        return run_output("wmctrl", &["-i", "-a", &format!("{id:#x}")]).map(|_| ());
    }
    Err("no xdotool or wmctrl on PATH".to_string())
}

/// Close a window by id (port of `PostMessage(WM_CLOSE)`).
#[cfg(target_os = "linux")]
pub(crate) fn close_window_by_id(id: isize) -> Result<(), String> {
    if tool_present("xdotool") {
        return run_output("xdotool", &["windowclose", &id.to_string()]).map(|_| ());
    }
    if tool_present("wmctrl") {
        return run_output("wmctrl", &["-i", "-c", &format!("{id:#x}")]).map(|_| ());
    }
    Err("no xdotool or wmctrl on PATH".to_string())
}

/// Parse `wmctrl -l` output (`0x04200007  0  host  Window title…`)
/// into `(id, title)` pairs.
#[cfg(target_os = "linux")]
pub(crate) fn parse_wmctrl_list(out: &str) -> Vec<(isize, String)> {
    let mut windows = Vec::new();
    for line in out.lines() {
        let mut parts = line.split_whitespace();
        let (Some(id_hex), Some(_desktop), Some(_host)) =
            (parts.next(), parts.next(), parts.next())
        else {
            continue;
        };
        let title: String = parts.collect::<Vec<_>>().join(" ");
        if let Ok(id) = isize::from_str_radix(id_hex.trim_start_matches("0x"), 16) {
            windows.push((id, title));
        }
    }
    windows
}

#[cfg(target_os = "linux")]
pub(crate) fn tool_present(tool: &str) -> bool {
    std::process::Command::new("sh")
        .args(["-c", &format!("command -v {tool} >/dev/null 2>&1")])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[cfg(target_os = "linux")]
pub(crate) fn run_output(tool: &str, args: &[&str]) -> Result<String, String> {
    let output = std::process::Command::new(tool)
        .args(args)
        .output()
        .map_err(|e| format!("{tool} unavailable: {e}"))?;
    if !output.status.success() {
        return Err(format!("{tool} {args:?} failed"));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Port of `get_window_by_name` — first fuzzy match in Z-order, walking the
/// `GW_HWNDNEXT` chain from `FindWindow(None, None)`.
pub fn get_window_by_name(name: &str) -> Result<isize, String> {
    #[cfg(windows)]
    {
        use windows::core::PCWSTR;
        use windows::Win32::Foundation::HWND;
        use windows::Win32::UI::WindowsAndMessaging::{FindWindowW, GetWindow, GW_HWNDNEXT};

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
    #[cfg(target_os = "linux")]
    {
        match list_windows() {
            Ok(windows) => windows
                .into_iter()
                .find(|(_, title)| title_matches(title, name))
                .map(|(id, _)| id)
                .ok_or_else(|| format!("Window '{name}' not found")),
            Err(e) => {
                log().warning(&format!("get_window_by_name({name:?}): {e}"));
                Err(format!("Window '{name}' not found"))
            }
        }
    }
    #[cfg(not(any(windows, target_os = "linux")))]
    {
        log().warning(&format!(
            "get_window_by_name({name:?}): only supported on Windows and Linux"
        ));
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

    #[cfg(target_os = "linux")]
    #[test]
    fn wmctrl_list_parses_ids_and_titles() {
        let sample =
            "0x04200007  0  host  Konsole\n0x04200008  1  host  Firefox — YouTube\nbad line\n";
        assert_eq!(
            parse_wmctrl_list(sample),
            vec![
                (0x04200007, "Konsole".to_string()),
                (0x04200008, "Firefox — YouTube".to_string()),
            ]
        );
        assert!(parse_wmctrl_list("").is_empty());
    }
}
