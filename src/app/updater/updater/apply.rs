//! Update check + apply orchestration (extracted from `updater.rs`).

use serde_json::Value;

use super::download::{download_and_extract, fetch_latest_release};
use super::versions::compare_versions;
use crate::app::utils::{
    args::{get_args, raw_args},
    exit::exit_program,
    logger::updater_log,
    settings::get_config::get_config,
    working_dir::get_base_dir,
};

/// Port of `check_updates`.
pub async fn check_updates(current_version: &str) {
    let wd_dir = get_base_dir();
    let update_dir = wd_dir.join("update");
    let settings = get_config(false, false)
        .get("settings")
        .cloned()
        .unwrap_or(Value::Null);

    let update_repo = settings
        .get("update_repo")
        .and_then(|v| v.as_str())
        .unwrap_or("Lenochxd/WebDeck");
    let update_channel = settings
        .get("update_channel")
        .and_then(|v| v.as_str())
        .unwrap_or("stable");

    let Some((latest_version, latest_release)) =
        fetch_latest_release(update_repo, update_channel).await
    else {
        updater_log().error("UPDATER: Could not fetch releases from GitHub.");
        return;
    };

    if !(compare_versions(&latest_version, current_version) > 0) && !get_args().force_update {
        updater_log().info("No updates available.");
        return;
    }

    if get_args().force_update {
        updater_log().info("Force update enabled.");
    }
    updater_log().info(&format!("New version available: {latest_version}"));

    exit_program(true, false);
    if let Some(assets) = latest_release.get("assets").and_then(|v| v.as_array()) {
        for asset in assets {
            let url = asset
                .get("browser_download_url")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let state = asset.get("state").and_then(|v| v.as_str()).unwrap_or("");
            if url.ends_with("portable.zip") && state == "uploaded" {
                download_and_extract(url, &wd_dir, &update_dir).await;
                break;
            }
        }
    }

    // Removing update files.
    for file_path in [
        update_dir.join("WebDeck"),
        update_dir.join("WD-update"),
        update_dir.join("WD-update.zip"),
    ] {
        if file_path.exists() {
            if file_path.is_dir() {
                let _ = std::fs::remove_dir_all(&file_path);
            } else {
                let _ = std::fs::remove_file(&file_path);
            }
        }
    }

    // Launch WebDeck from the root directory.
    updater_log().success("\nRestarting WebDeck.exe");
    let _ = std::env::set_current_dir(&wd_dir);
    #[cfg(windows)]
    let webdeck_path = wd_dir.join("WebDeck.exe");
    #[cfg(not(windows))]
    let webdeck_path = wd_dir.join("WebDeck");
    let mut command = std::process::Command::new(&webdeck_path);
    if compare_versions(&latest_version, "2.0.0") >= 0 {
        command.args(raw_args());
    }
    match command.spawn() {
        Ok(_) => {}
        Err(e) => updater_log().exception(
            &e,
            Some(&format!("Failed to relaunch {}", webdeck_path.display())),
            true,
            true,
            true,
        ),
    }
}
