//! Port of `app/updater/updater.py`.
//!
//! - `requests` → `reqwest` (async fns), `tqdm` → log lines, `zipfile` → `zip`.
//! - `compare_versions` / `check_files` / `move_folder_content` are ported 1:1.
//! - Logging goes to the updater log file ([`updater_log`]), like Python's
//!   module-level `Logger(from_updater=True)`.
//! - Python uses `__main__`-time globals (`settings`, `wd_dir`, `update_dir`);
//!   Rust recomputes them from [`get_base_dir`] in each function.

use serde_json::Value;

use crate::app::utils::{
    args::{get_args, raw_args},
    exit::exit_program,
    logger::updater_log,
    settings::get_config::get_config,
    show_error::show_error,
    working_dir::get_base_dir,
};

/// Port of `check_files`.
pub fn check_files() {
    let wd_dir = get_base_dir();
    let version_path = wd_dir.join("webdeck").join("version.json");
    let temp_json_path = wd_dir.join("temp.json");

    let current_version = std::fs::read_to_string(&version_path)
        .ok()
        .and_then(|content| serde_json::from_str::<Value>(&content).ok())
        .and_then(|versions| {
            versions
                .get("versions")?
                .get(0)?
                .get("version")?
                .as_str()
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| "0.0.0".to_string());

    let mut temp_json: Value = std::fs::read_to_string(&temp_json_path)
        .ok()
        .and_then(|content| serde_json::from_str(&content).ok())
        .unwrap_or(Value::Object(serde_json::Map::new()));
    if temp_json.get("checked-versions").is_none() {
        temp_json["checked-versions"] = Value::Array(Vec::new());
    }

    let versions: Vec<Value> = std::fs::read_to_string(&version_path)
        .ok()
        .and_then(|content| serde_json::from_str::<Value>(&content).ok())
        .and_then(|v| v.get("versions").cloned())
        .and_then(|v| v.as_array().cloned())
        .unwrap_or_default();

    for version in versions.iter().rev() {
        let version_name = version
            .get("version")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let already_checked = temp_json
            .get("checked-versions")
            .and_then(|v| v.as_array())
            .map(|a| a.iter().any(|v| v.as_str() == Some(version_name.as_str())))
            .unwrap_or(false);
        if already_checked {
            continue;
        }
        if let Some(checked) = temp_json
            .get_mut("checked-versions")
            .and_then(|v| v.as_array_mut())
        {
            checked.push(Value::String(version_name));
        }

        // Handle deleted files.
        if let Some(deleted) = version.get("deleted_files").and_then(|v| v.as_array()) {
            for file_entry in deleted {
                let (file_to_delete, update_limit) = match file_entry {
                    Value::String(name) => (name.clone(), "99.99.99".to_string()),
                    Value::Array(pair) => (
                        pair.get(0).and_then(|v| v.as_str()).unwrap_or_default().to_string(),
                        pair.get(1).and_then(|v| v.as_str()).unwrap_or("99.99.99").to_string(),
                    ),
                    _ => continue,
                };
                if compare_versions(&update_limit, &current_version) > 0 {
                    let path = wd_dir.join(&file_to_delete);
                    match std::fs::remove_file(&path) {
                        Ok(()) => updater_log()
                            .info(&format!("UPDATER: Deleted {}", path.display())),
                        Err(e) => updater_log().exception(
                            &e,
                            Some(&format!("UPDATER: Error deleting {}", path.display())),
                            true,
                            true,
                            true,
                        ),
                    }
                }
            }
        }

        // Handle moved or renamed files.
        let mut files_to_move: Vec<&Value> = Vec::new();
        for key in ["moved_files", "renamed_files"] {
            if let Some(list) = version.get(key).and_then(|v| v.as_array()) {
                files_to_move.extend(list.iter());
            }
        }
        for entry in files_to_move {
            let Some(pair) = entry.as_array() else {
                continue;
            };
            let source = wd_dir.join(pair.get(0).and_then(|v| v.as_str()).unwrap_or_default());
            let destination =
                wd_dir.join(pair.get(1).and_then(|v| v.as_str()).unwrap_or_default());
            let update_limit = pair
                .get(2)
                .and_then(|v| v.as_str())
                .unwrap_or("99.99.99");
            if compare_versions(update_limit, &current_version) > 0 {
                if let Some(parent) = destination.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                match std::fs::rename(&source, &destination) {
                    Ok(()) => updater_log().info(&format!(
                        "Moved {} -> {}",
                        source.display(),
                        destination.display()
                    )),
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                        updater_log().error(&format!("File not found: {}", source.display()));
                    }
                    Err(e) => updater_log().exception(
                        &e,
                        Some(&format!("UPDATER: Error moving {}", source.display())),
                        true,
                        true,
                        true,
                    ),
                }
            }
        }
    }

    let _ = std::fs::write(
        &temp_json_path,
        serde_json::to_string_pretty(&temp_json).unwrap_or_default(),
    );
}

/// Port of `move_folder_content` (progress bar replaced by a summary log).
pub fn move_folder_content(source: &std::path::Path, destination: &std::path::Path) {
    if !destination.exists() {
        let _ = std::fs::create_dir_all(destination);
    }

    let mut elements: Vec<std::path::PathBuf> = Vec::new();
    let mut stack = vec![source.to_path_buf()];
    while let Some(dir) = stack.pop() {
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path.clone());
                }
                elements.push(path);
            }
        }
    }

    let mut moved = 0;
    for element in &elements {
        let Ok(relative) = element.strip_prefix(source) else {
            continue;
        };
        let destination_path = destination.join(relative);
        if element.is_file() {
            if let Some(parent) = destination_path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if std::fs::copy(element, &destination_path).is_ok() {
                moved += 1;
            }
        } else if element.is_dir() {
            let _ = std::fs::create_dir_all(&destination_path);
            moved += 1;
        }
    }
    updater_log().info(&format!("Moving files: {moved}/{} done", elements.len()));
}

/// Port of the nested `parse_version` + `compare_versions`.
///
/// Returns 1 if `version1` > `version2`, -1 if less, 0 if equal.
/// `-pre`/`-beta` suffixes sort below the plain release.
pub fn compare_versions(version1: &str, version2: &str) -> i32 {
    fn parse_version(version: &str) -> (Vec<u64>, Option<&str>) {
        if let Some(base) = version.strip_suffix("-pre") {
            return (base.split('.').filter_map(|p| p.parse().ok()).collect(), Some("pre"));
        }
        if let Some(base) = version.strip_suffix("-beta") {
            return (base.split('.').filter_map(|p| p.parse().ok()).collect(), Some("beta"));
        }
        (version.split('.').filter_map(|p| p.parse().ok()).collect(), None)
    }

    let (v1, s1) = parse_version(version1);
    let (v2, s2) = parse_version(version2);

    for (a, b) in v1.iter().zip(v2.iter()) {
        if a != b {
            return if a > b { 1 } else { -1 };
        }
    }
    if v1.len() != v2.len() {
        return if v1.len() > v2.len() { 1 } else { -1 };
    }

    let rank = |suffix: Option<&str>| match suffix {
        Some("pre") | Some("beta") => -1,
        _ => 0,
    };
    rank(s1) - rank(s2)
}

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
            let draft = release.get("draft").and_then(|v| v.as_bool()).unwrap_or(false);
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
    updater_log().info(&format!("Downloading: {downloaded}/{total_size} bytes done"));

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
        use windows::Win32::UI::Shell::{IsUserAnAdmin, ShellExecuteW};
        use windows::Win32::UI::WindowsAndMessaging::SW_NORMAL;
        use windows::core::{w, HSTRING};

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_ordering() {
        assert_eq!(compare_versions("1.8.7", "1.8.7"), 0);
        assert_eq!(compare_versions("1.8.8", "1.8.7"), 1);
        assert_eq!(compare_versions("1.8.6", "1.8.7"), -1);
        assert_eq!(compare_versions("2.0.0", "1.99.99"), 1);
        assert_eq!(compare_versions("1.8.7", "1.8"), 1);
        assert_eq!(compare_versions("1.8.7-beta", "1.8.7"), -1);
        assert_eq!(compare_versions("1.8.7-pre", "1.8.7-beta"), 0);
        assert_eq!(compare_versions("1.8.8-beta", "1.8.7"), 1);
    }
}
