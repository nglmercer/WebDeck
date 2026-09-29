//! Port of `app/updater/check.py`.
//!
//! `requests` maps to `reqwest` (async — hence these are async fns). Like
//! Python (which returns early unless frozen), debug builds skip the update
//! check; release builds behave like frozen runs.

use serde_json::Value;

use crate::app::updater::{compare_versions, fetch_latest_release, prepare_update_directory};
use crate::app::utils::{
    args::{get_args, raw_args},
    exit::exit_program,
    languages::text,
    logger::log,
    settings::get_config::get_config,
    show_error::show_error,
    working_dir::get_base_dir,
};

/// Port of `check_for_updates`.
pub async fn check_for_updates() {
    if cfg!(debug_assertions) {
        return;
    }

    let config = get_config(false, false);
    let settings = config.get("settings").cloned().unwrap_or(Value::Null);

    if std::path::Path::new("update").exists() {
        let _ = std::fs::remove_dir_all("update");
    }

    let result = check_for_updates_inner(&settings).await;
    if let Err(message) = result {
        log().exception(
            &message,
            Some("UPDATER: Error occurred while checking for updates"),
            true,
            true,
            true,
        );
        // Native error dialogs block: run off the async worker.
        tokio::task::block_in_place(|| {
            show_error(
                Some(&format!(
                    "{} \n\n{}: {message}",
                    text(Some("auto_update_error"), None),
                    text(Some("error"), None)
                )),
                "WebDeck Updater Error",
                true,
                Some(&message as &dyn std::fmt::Debug),
            );
        });
    }
}

async fn check_for_updates_inner(settings: &Value) -> Result<(), String> {
    let content = std::fs::read_to_string("webdeck/version.json")
        .map_err(|e| format!("Cannot read webdeck/version.json: {e}"))?;
    let versions: Value =
        serde_json::from_str(&content).map_err(|e| format!("Cannot parse version.json: {e}"))?;
    let current_version = versions
        .get("versions")
        .and_then(|v| v.get(0))
        .and_then(|v| v.get("version"))
        .and_then(|v| v.as_str())
        .unwrap_or("0.0.0")
        .to_string();

    let update_repo = settings
        .get("update_repo")
        .and_then(|v| v.as_str())
        .unwrap_or("Lenochxd/WebDeck");
    let update_channel = settings
        .get("update_channel")
        .and_then(|v| v.as_str())
        .unwrap_or("stable");

    let latest_version = fetch_latest_release(update_repo, update_channel)
        .await
        .map(|(version, _)| version)
        .unwrap_or_else(|| "1.0.0".to_string());

    let is_new_version_available = compare_versions(&latest_version, &current_version) > 0;
    let args = get_args();

    if (is_new_version_available || args.force_update) && !args.no_auto_update {
        log().info(&format!("UPDATER: New version available: {latest_version}"));
        prepare_update_directory();

        // Python chdirs into update/ and spawns the updater binary with the
        // original CLI args, then exits; mirrored here.
        let _ = std::env::set_current_dir("update");
        #[cfg(windows)]
        let updater_bin = get_base_dir().join("update").join("update.exe");
        #[cfg(not(windows))]
        let updater_bin = get_base_dir().join("update").join("update");
        match std::process::Command::new(&updater_bin)
            .args(raw_args())
            .spawn()
        {
            Ok(_) => {}
            Err(e) => {
                return Err(format!("Failed to launch {}: {e}", updater_bin.display()));
            }
        }
        exit_program(true, false);
    }

    Ok(())
}

/// Port of `check_for_updates_loop`.
pub async fn check_for_updates_loop() {
    loop {
        let config = get_config(false, false);
        let args = get_args();
        // NOTE: Python reads the legacy "auto-updates" key here (hyphenated);
        // kept 1:1 (check_config_update normalizes stored configs anyway).
        let auto_updates = config
            .get("settings")
            .and_then(|s| s.get("auto-updates"))
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        if (auto_updates || args.force_update) && !args.no_auto_update {
            check_for_updates().await;
        }
        tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
    }
}
