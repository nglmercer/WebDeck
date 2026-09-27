//! Updater staging + elevation (extracted from `updater.rs`).

use crate::app::utils::{logger::updater_log, working_dir::get_base_dir};
use crate::app::utils::args::get_args;
#[cfg(windows)]
use crate::app::utils::args::raw_args;

/// Port of `prepare_update_directory` — stages the updater binary into
/// `<base>/update/` so `check_for_updates` can relaunch from there.
///
/// Python additionally stages `python3*.dll` + the `lib/` tree (cx_Freeze
/// runtime); those steps are N/A for the statically-linked Rust binary.
pub fn prepare_update_directory() {
    let current_dir = std::env::current_dir().unwrap_or_else(|_| ".".into());

    let wd_dir = get_base_dir();
    let update_dir = wd_dir.join("update");
    let _ = std::fs::create_dir_all(&update_dir);

    #[cfg(windows)]
    let binary_name = "update.exe";
    #[cfg(not(windows))]
    let binary_name = "update";

    let staged = update_dir.join(binary_name);
    if !staged.exists() {
        // Prefer the installed updater next to the app binary, else the running exe.
        let candidates = [
            wd_dir.join(binary_name),
            std::env::current_exe().unwrap_or_default(),
        ];
        for candidate in candidates {
            if candidate.is_file() {
                match std::fs::copy(&candidate, &staged) {
                    Ok(_) => {
                        updater_log().info(&format!(
                            "Staged updater: {} -> {}",
                            candidate.display(),
                            staged.display()
                        ));
                        break;
                    }
                    Err(e) => updater_log().exception(
                        &e,
                        Some(&format!("Cannot stage {}", staged.display())),
                        true,
                        true,
                        true,
                    ),
                }
            }
        }
    }

    let _ = std::env::set_current_dir(&current_dir);
}

/// Port of `needs_admin_permissions` — true when the install dir is not
/// writable by this process (same probe files as Python, minus the
/// cx_Freeze-only `python3.dll` copy, which is N/A for static binaries).
pub fn needs_admin_permissions() -> bool {
    #[cfg(windows)]
    {
        let base = get_base_dir();
        let test_file = base.join("test_admin.txt");
        let staged_probe = base.join("update").join("test_admin.txt");

        let probe = (|| -> std::io::Result<()> {
            std::fs::write(&test_file, b"This is a test.")?;
            std::fs::remove_file(&test_file)?;
            std::fs::create_dir_all(base.join("update"))?;
            std::fs::write(&staged_probe, b"This is a test.")?;
            std::fs::remove_file(&staged_probe)?;
            Ok(())
        })();

        match probe {
            Ok(()) => false,
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => true,
            Err(e) => {
                updater_log().error(&format!(
                    "Unexpected error during admin permission check: {e}"
                ));
                true
            }
        }
    }
    #[cfg(not(windows))]
    {
        false
    }
}

/// Port of `request_admin_permissions`.
///
/// Honors `--no-admin`. Otherwise relaunches elevated via `runas` and exits
/// (Windows); non-Windows logs and continues without elevation.
pub fn request_admin_permissions() {
    if get_args().no_admin {
        updater_log().info("Skipping admin permissions request due to '--no-admin' argument.");
        return;
    }

    #[cfg(windows)]
    {
        use windows::core::{w, HSTRING};
        use windows::Win32::UI::Shell::{IsUserAnAdmin, ShellExecuteW};
        use windows::Win32::UI::WindowsAndMessaging::SW_NORMAL;

        let is_admin = unsafe { IsUserAnAdmin().as_bool() };
        if !is_admin {
            let exe = std::env::current_exe()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();
            let params = raw_args().join(" ");
            unsafe {
                let _ = ShellExecuteW(
                    None,
                    w!("runas"),
                    &HSTRING::from(exe.as_str()),
                    &HSTRING::from(params.as_str()),
                    None,
                    SW_NORMAL,
                );
            }
            std::process::exit(0);
        }
    }
    #[cfg(not(windows))]
    {
        updater_log().warning(
            "Admin permissions needed, but UAC relaunch is Windows-only; continuing without elevation",
        );
    }
}
