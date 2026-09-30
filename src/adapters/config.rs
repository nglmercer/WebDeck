use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::domain::error::{AppError, ErrorCode};

static NEXT_FILE: AtomicU64 = AtomicU64::new(0);

fn failure() -> AppError {
    AppError::new(ErrorCode::PersistenceFailed, "Cannot persist configuration")
}

pub fn protected_write(path: &Path, content: &[u8], create_new: bool) -> Result<(), AppError> {
    let mut options = OpenOptions::new();
    options.write(true);
    if create_new {
        options.create_new(true);
    } else {
        options.create(true).truncate(true);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path).map_err(|_| failure())?;
    file.write_all(content).map_err(|_| failure())?;
    file.sync_all().map_err(|_| failure())
}

/// Unique sibling files avoid collisions between independent processes.
/// Publication occurs only after a durable write; failures leave the old file.
pub fn atomic_replace(path: &Path, content: &[u8]) -> Result<(), AppError> {
    let temporary = path.with_extension(format!(
        "{}.{}.tmp",
        std::process::id(),
        NEXT_FILE.fetch_add(1, Ordering::Relaxed)
    ));
    let result = protected_write(&temporary, content, true)
        .and_then(|()| fs::rename(&temporary, path).map_err(|_| failure()));
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result?;
    #[cfg(unix)]
    if let Some(parent) = path.parent() {
        // Once rename succeeds, the new file is authoritative even if a
        // filesystem cannot fsync directories (never publish a stale cache).
        if let Ok(dir) = fs::File::open(parent) {
            let _ = dir.sync_all();
        }
    }
    Ok(())
}

/// Advisory process-wide writer serialization for supported WebDeck owners.
/// External editors do not honor this lock; fingerprint checks still detect
/// their changes before publication. Dropping File releases the OS lock.
pub fn exclusive_lock(path: &Path) -> Result<fs::File, AppError> {
    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600).custom_flags(libc::O_NOFOLLOW);
    }
    let file = options
        .open(path.with_extension("writer.lock"))
        .map_err(|_| failure())?;
    file.lock().map_err(|_| failure())?;
    Ok(file)
}
