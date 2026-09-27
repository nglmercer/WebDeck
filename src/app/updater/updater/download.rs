//! Release download + extraction (extracted from `updater.rs`).

use serde_json::Value;

use super::files::move_folder_content;
use crate::app::utils::{logger::updater_log, show_error::show_error};

/// Shared GitHub-release lookup used by `check_updates` (and the
/// near-identical lookup in `updater::check`). Returns
/// `(latest_version, latest_release)`.
pub async fn fetch_latest_release(
    update_repo: &str,
    update_channel: &str,
) -> Option<(String, Value)> {
    let url = format!("https://api.github.com/repos/{update_repo}/releases");
    let client = reqwest::Client::builder()
        .user_agent("WebDeck")
        .build()
        .ok()?;
    let releases: Value = client.get(&url).send().await.ok()?.json().await.ok()?;
    let releases = releases.as_array()?;

    let latest = releases
        .iter()
        .find(|release| {
            let draft = release
                .get("draft")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let prerelease = release
                .get("prerelease")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            !draft
                && ((update_channel == "stable" && !prerelease)
                    || (update_channel == "beta" && prerelease))
        })
        .cloned()
        .unwrap_or(serde_json::json!({"tag_name": "v1.0.0"}));

    let version = latest
        .get("tag_name")
        .and_then(|v| v.as_str())
        .unwrap_or("v1.0.0")
        .replace('v', "");
    Some((version, latest))
}

/// Port of `download_and_extract`.
pub async fn download_and_extract(
    download_url: &str,
    wd_dir: &std::path::Path,
    update_dir: &std::path::Path,
) {
    let client = reqwest::Client::builder()
        .user_agent("WebDeck")
        .build()
        .expect("Cannot build HTTP client");
    let response = match client.get(download_url).send().await {
        Ok(response) => response,
        Err(e) => {
            show_error(
                Some(&format!("Failed to download update ZIP file.\n\n{e}")),
                "WebDeck Updater Error",
                true,
                Some(&e as &dyn std::fmt::Debug),
            );
            return;
        }
    };
    if !response.status().is_success() {
        let body = response.text().await.unwrap_or_default();
        show_error(
            Some(&format!("Failed to download update ZIP file.\n\n{body}")),
            "WebDeck Updater Error",
            true,
            None,
        );
        return;
    }

    let total_size: u64 = response
        .headers()
        .get(reqwest::header::CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);

    let mut file = match std::fs::File::create("WD-update.zip") {
        Ok(file) => file,
        Err(e) => {
            updater_log().exception(&e, Some("Cannot create WD-update.zip"), true, true, true);
            return;
        }
    };
    let mut downloaded: u64 = 0;
    let mut response = response;
    loop {
        match response.chunk().await {
            Ok(Some(chunk)) => {
                use std::io::Write;
                downloaded += chunk.len() as u64;
                if file.write_all(&chunk).is_err() {
                    break;
                }
            }
            Ok(None) => break,
            Err(e) => {
                updater_log().exception(&e, Some("Download interrupted"), true, true, true);
                break;
            }
        }
    }
    drop(file);
    updater_log().info(&format!(
        "Downloading: {downloaded}/{total_size} bytes done"
    ));

    if total_size != 0 && downloaded != total_size {
        let percentage = (downloaded as f64 / total_size as f64) * 100.0;
        show_error(
            Some(&format!(
                "Failed to download the complete update ZIP file.\n\nDownloaded: {downloaded}\nExpected: {total_size}\nDifference: {}\nPercentage: {percentage:.1}%",
                total_size as i64 - downloaded as i64
            )),
            "WebDeck Updater Error",
            true,
            None,
        );
        return;
    }

    let zip_file = match std::fs::File::open("WD-update.zip") {
        Ok(file) => file,
        Err(e) => {
            updater_log().exception(&e, Some("Cannot open WD-update.zip"), true, true, true);
            return;
        }
    };
    let mut archive = match zip::ZipArchive::new(zip_file) {
        Ok(archive) => archive,
        Err(e) => {
            updater_log().exception(&e, Some("Cannot read update ZIP"), true, true, true);
            return;
        }
    };
    updater_log().info(&format!("Extracting: {} files", archive.len()));
    if let Err(e) = archive.extract("WD-update") {
        updater_log().exception(&e, Some("Cannot extract update ZIP"), true, true, true);
        return;
    }

    move_folder_content(&update_dir.join("WD-update").join("WebDeck"), wd_dir);
}
