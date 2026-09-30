//! Update check + apply orchestration (extracted from `updater.rs`).

use serde_json::Value;

use super::download::fetch_latest_release;
use super::versions::compare_versions;
use crate::app::utils::{
    args::get_args, logger::updater_log, settings::get_config::get_config,
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

    // Select this exact platform. Missing digest fails closed, retaining the
    // current installation; historical unverified releases are not applied.
    let wanted = format!(
        "WebDeck-{}-{}-portable.zip",
        std::env::consts::OS,
        std::env::consts::ARCH
    );
    let asset = latest_release["assets"].as_array().and_then(|assets| {
        assets.iter().find(|a| {
            a["name"].as_str() == Some(&wanted) && a["state"].as_str() == Some("uploaded")
        })
    });
    let Some(asset) = asset else {
        updater_log().error("No portable artifact for this platform");
        return;
    };
    let Some(expected) = asset["digest"].as_str() else {
        updater_log().error("Update lacks a trusted SHA-256 digest; installation unchanged");
        return;
    };
    let url = asset["browser_download_url"].as_str().unwrap_or("");
    let result = async {
        let bytes = crate::adapters::update::download(url).await?;
        std::fs::create_dir_all(&update_dir)?;
        let mut random = [0u8; 8];
        getrandom::fill(&mut random).map_err(std::io::Error::other)?;
        let identifier: String = random.iter().map(|b| format!("{b:02x}")).collect();
        let stage = update_dir.join(format!("staged-{identifier}"));
        let backup = update_dir.join(format!("backup-{identifier}"));
        let staged = crate::adapters::update::stage(&bytes, expected, &stage)?;
        // Replacement is attempted only after verification. Locked files fail
        // with rollback rather than killing unrelated processes by image name.
        crate::adapters::update::install(&staged, &wd_dir, &backup)?;
        std::fs::remove_dir_all(staged)?;
        Ok::<(), std::io::Error>(())
    }
    .await;
    if result.is_err() {
        updater_log()
            .error("Update failed; retain the installation and inspect the recovery backup");
        return;
    }
    updater_log().success("Verified update installed; backup retained under update/");
    #[cfg(windows)]
    let webdeck_path = wd_dir.join("WebDeck.exe");
    #[cfg(not(windows))]
    let webdeck_path = wd_dir.join("WebDeck");
    let result = std::process::Command::new(webdeck_path)
        .current_dir(wd_dir)
        .args(std::env::args().skip(1))
        .spawn();
    if result.is_err() {
        updater_log()
            .error("Updated application could not start; use the retained backup to roll back");
    }
}
