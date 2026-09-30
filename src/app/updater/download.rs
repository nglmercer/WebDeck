//! Release download + extraction (extracted from `updater.rs`).

use serde_json::Value;

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
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .ok()?;
    let releases: Value = client
        .get(&url)
        .send()
        .await
        .ok()?
        .error_for_status()
        .ok()?
        .json()
        .await
        .ok()?;
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
