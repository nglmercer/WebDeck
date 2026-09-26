//! Port of `app/utils/kill_nircmd.py`.

/// Port of `kill_nircmd` (errors intentionally ignored, like Python).
pub fn kill_nircmd() {
    #[cfg(windows)]
    {
        let _ = std::process::Command::new("taskkill")
            .args(["/f", "/IM", "nircmd.exe"])
            .spawn();
    }
    #[cfg(not(windows))]
    {
        // nircmd is Windows-only; no-op elsewhere.
    }
}
