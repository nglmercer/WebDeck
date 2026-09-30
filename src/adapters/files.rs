//! Configuration-addressable assets retain their URL paths, while their
//! physical root follows WEBDECK_CONFIG_DIR. No symlinks or traversal.
use crate::domain::error::{AppError, ErrorCode};
use std::path::{Path, PathBuf};

pub fn confined_path(root: &Path, directory: &str, filename: &str) -> Result<PathBuf, AppError> {
    let invalid = || AppError::new(ErrorCode::InvalidInput, "Invalid asset path");
    if !matches!(directory, "user_uploads" | "themes" | "plugins")
        || filename.is_empty()
        || filename.len() > 255
        || matches!(filename, "." | "..")
        || filename
            .chars()
            .any(|c| c.is_control() || matches!(c, '/' | '\\' | ':' | '"'))
    {
        return Err(invalid());
    }
    let base = root.canonicalize().map_err(|_| invalid())?;
    let directory = root.join(directory).canonicalize().map_err(|_| invalid())?;
    if !directory.starts_with(&base) {
        return Err(invalid());
    }
    let path = directory.join(filename);
    if std::fs::symlink_metadata(&path).is_ok_and(|m| m.file_type().is_symlink()) {
        return Err(invalid());
    }
    Ok(path)
}

pub fn read_asset(path: &Path) -> Result<Vec<u8>, AppError> {
    use std::io::Read;
    let failed = || AppError::new(ErrorCode::PersistenceFailed, "Cannot read asset");
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW);
    }
    let file = options.open(path).map_err(|_| failed())?;
    let mut bytes = Vec::new();
    file.take(16 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| failed())?;
    if bytes.len() > 16 * 1024 * 1024 {
        return Err(failed());
    }
    Ok(bytes)
}

pub fn resolve_legacy_asset(path: &str) -> PathBuf {
    let normalized = path.replace('\\', "/");
    if let Some(relative) = normalized.strip_prefix(".config/") {
        crate::app::utils::settings::get_config::config_dir().join(relative)
    } else {
        PathBuf::from(path)
    }
}
