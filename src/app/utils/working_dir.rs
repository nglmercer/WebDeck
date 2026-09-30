//! Port of `app/utils/working_dir.py`.
//!
//! Python keys its behavior off `sys.frozen` (cx_Freeze builds). The Rust
//! equivalent is the build profile: debug builds behave like unfrozen runs
//! (current directory), release builds search upward for the installed
//! layout like frozen runs.

use std::path::{Path, PathBuf};

/// Port of `locate_file_directory`.
///
/// In debug builds (unfrozen equivalent) returns the current directory.
/// Otherwise searches `base_dir` (or cwd) and the executable's directory,
/// walking up to 4 parents for `filename`.
pub fn locate_file_directory(filename: &str, base_dir: Option<&Path>) -> PathBuf {
    if cfg!(debug_assertions) {
        return std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    }

    let mut search_dirs = Vec::new();
    search_dirs.push(
        base_dir
            .map(|p| p.to_path_buf())
            .or_else(|| std::env::current_dir().ok())
            .unwrap_or_else(|| PathBuf::from(".")),
    );
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            search_dirs.push(parent.to_path_buf());
        }
    }

    for search_dir in search_dirs {
        let mut current = search_dir;
        for _ in 0..5 {
            if current.join(filename).exists() {
                return current;
            }
            match current.parent() {
                Some(parent) => current = parent.to_path_buf(),
                None => break,
            }
        }
    }

    // Python raises FileNotFoundError here; fall back to the executable's
    // directory so release builds on non-Windows layouts still start.
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
}

/// Port of `get_base_dir`.
pub fn get_base_dir() -> PathBuf {
    locate_file_directory(&format!("WebDeck{}", std::env::consts::EXE_SUFFIX), None)
}

/// Port of `get_update_dir`.
pub fn get_update_dir() -> PathBuf {
    let base_dir = get_base_dir().join("update");
    locate_file_directory(
        &format!("update{}", std::env::consts::EXE_SUFFIX),
        Some(&base_dir),
    )
}

/// Port of `chdir_base`.
pub fn chdir_base() {
    let base = get_base_dir();
    let _ = std::env::set_current_dir(&base);
}

/// Port of `chdir_update`.
pub fn chdir_update() {
    let dir = get_update_dir();
    let _ = std::env::set_current_dir(&dir);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_build_uses_cwd() {
        if cfg!(debug_assertions) {
            assert_eq!(
                locate_file_directory(&format!("WebDeck{}", std::env::consts::EXE_SUFFIX), None),
                std::env::current_dir().unwrap()
            );
        }
    }
}
