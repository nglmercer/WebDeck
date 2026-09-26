//! Updater binary — port of the `__main__` block in `app/updater/updater.py`.
//!
//! Python only runs this block when frozen (`sys.frozen`); the Rust equivalent
//! is a release build. In debug builds we still run it so the flow is testable,
//! and log that fact instead of silently returning.
//!
//! Run with `cargo run --bin update`.

#![allow(dead_code)]

use webdeck::app::updater::updater as updater_mod;
use webdeck::app::utils::{args, logger::log, working_dir};

#[tokio::main]
async fn main() {
    if cfg!(debug_assertions) {
        log().debug("Running updater in debug mode (Python skips this unless frozen)");
    }

    log().info("Starting updater...");

    args::parse_args();

    let wd_dir = working_dir::get_base_dir();
    working_dir::chdir_update();

    let in_update_dir = std::env::current_dir()
        .map(|cwd| {
            cwd.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n == "update")
                .unwrap_or(false)
        })
        .unwrap_or(false);

    if !in_update_dir {
        log().info("Preparing update directory...");

        if updater_mod::needs_admin_permissions() {
            updater_mod::request_admin_permissions();
        }

        updater_mod::prepare_update_directory();
        log().info("Launching update binary...");

        #[cfg(windows)]
        let update_exe_path = wd_dir.join("update").join("update.exe");
        #[cfg(not(windows))]
        let update_exe_path = wd_dir.join("update").join("update");

        match std::process::Command::new(&update_exe_path)
            .args(args::raw_args())
            .spawn()
        {
            Ok(_) => {}
            Err(e) => log().exception(
                &e,
                Some(&format!("Failed to launch {}", update_exe_path.display())),
                true,
                true,
                true,
            ),
        }
        return;
    }

    if updater_mod::needs_admin_permissions() {
        updater_mod::request_admin_permissions();
    }

    let version_path = wd_dir.join("webdeck").join("version.json");
    let current_version = std::fs::read_to_string(&version_path)
        .ok()
        .and_then(|content| serde_json::from_str::<serde_json::Value>(&content).ok())
        .and_then(|v| {
            v.get("versions")?
                .get(0)?
                .get("version")?
                .as_str()
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| "0.0.0".to_string());

    updater_mod::check_files();
    updater_mod::check_updates(&current_version).await;
}
