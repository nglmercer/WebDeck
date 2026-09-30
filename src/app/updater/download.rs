//! Release download + extraction (extracted from `updater.rs`).

use serde_json::Value;

/// Shared GitHub-release lookup used by `check_updates` (and the
/// near-identical lookup in `updater::check`). Returns
/// `(latest_version, latest_release)`.
pub async fn fetch_latest_release(
    update_repo: &str,
    update_channel: &str,
) -> Option<(String, Value)> {
    if update_repo != "nglmercer/WebDeck" || update_channel != "v2-prerelease" {
        return None;
    }
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
        .find(|release| eligible_release(release))?
        .clone();
    let version = latest["tag_name"].as_str()?.strip_prefix('v')?.to_string();
    Some((version, latest))
}

fn eligible_release(release: &Value) -> bool {
    release["draft"].as_bool() == Some(false)
        && release["prerelease"].as_bool() == Some(true)
        && release["tag_name"].as_str().is_some_and(|tag| {
            // Accept only the v2 prerelease line. Do not synthesize a v1
            // release when the maintainer has not published a candidate.
            tag.strip_prefix('v')
                .and_then(|v| semver::Version::parse(v).ok())
                .is_some_and(|v| v.major == 2 && !v.pre.is_empty())
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn only_maintainer_v2_prereleases_are_candidates() {
        for (tag, prerelease, draft, allowed) in [
            ("v2.0.0-alpha.1", true, false, true),
            ("v1.8.7-beta", true, false, false),
            ("v2.0.0", false, false, false),
            ("v2.0.0-alpha.1", true, true, false),
            ("v20.0.0-alpha.1", true, false, false),
            ("v2.x.0-alpha.1", true, false, false),
        ] {
            assert_eq!(
                eligible_release(&json!({"tag_name":tag,"prerelease":prerelease,"draft":draft})),
                allowed
            );
        }
    }
}
