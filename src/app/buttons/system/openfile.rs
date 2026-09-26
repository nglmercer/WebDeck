//! Port of `app/buttons/system/openfile.py`.
//!
//! `os.startfile` maps to `cmd /C start` on Windows, `xdg-open` on Linux,
//! `open` on macOS. The chdir-into-parent behavior for drive-letter paths is
//! kept 1:1.

use crate::app::utils::logger::log;

/// Port of `openfile`.
pub fn openfile(path: &str) {
    if !path.contains("://") && path.contains(':') {
        let initial = std::env::current_dir().unwrap_or_else(|_| ".".into());
        let parent = std::path::Path::new(path)
            .parent()
            .map(|p| p.to_path_buf());
        if let Some(parent) = parent {
            let _ = std::env::set_current_dir(&parent);
        }
        startfile(path);
        let _ = std::env::set_current_dir(&initial);
    } else {
        startfile(path);
    }
}

fn startfile(path: &str) {
    #[cfg(windows)]
    let spawned = std::process::Command::new("cmd")
        .args(["/C", "start", "", path])
        .spawn();
    #[cfg(target_os = "macos")]
    let spawned = std::process::Command::new("open").arg(path).spawn();
    #[cfg(not(any(windows, target_os = "macos")))]
    let spawned = std::process::Command::new("xdg-open").arg(path).spawn();

    if let Err(e) = spawned {
        log().error(&format!("openfile({path:?}) failed: {e}"));
    }
}
