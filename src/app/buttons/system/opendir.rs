//! Port of `app/buttons/system/opendir.py`.

use crate::app::utils::logger::log;

fn normalize_path(path: &str) -> String {
    path.replace("\\\\", "\\").replace('\\', "/")
}

fn absolute(path: &str) -> String {
    let candidate = std::path::PathBuf::from(path);
    if candidate.is_absolute() {
        return normalize_path(path);
    }
    let cwd = std::env::current_dir().unwrap_or_else(|_| ".".into());
    normalize_path(&cwd.join(candidate).to_string_lossy())
}

/// Port of `opendir` (returns the opened path, like Python).
pub fn opendir(message: &str) -> String {
    let path = message.replacen("/openfolder", "", 1).replacen("/opendir", "", 1);
    let path = absolute(normalize_path(path.trim()).as_str());

    log().debug(&format!("Opening directory: {path}"));

    #[cfg(windows)]
    let spawned = std::process::Command::new("explorer")
        .arg(format!("\"{path}\""))
        .spawn();
    #[cfg(target_os = "macos")]
    let spawned = std::process::Command::new("open").arg(&path).spawn();
    #[cfg(not(any(windows, target_os = "macos")))]
    let spawned = std::process::Command::new("xdg-open").arg(&path).spawn();

    if let Err(e) = spawned {
        log().error(&format!("opendir({path:?}) failed: {e}"));
    }

    path
}
