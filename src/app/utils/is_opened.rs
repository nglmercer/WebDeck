//! Port of `app/utils/is_opened.py`.
//!
//! Python keys "frozen" off cx_Freeze; Rust uses the build profile
//! (`cfg!(debug_assertions)` ⇔ unfrozen). Process enumeration uses `tasklist`
//! on Windows and `/proc` on Linux (no `psutil` equivalent needed).

use crate::app::utils::logger::log;

/// Port of `is_opened`.
pub fn is_opened() -> bool {
    // Unfrozen equivalent: never single-instance-guarded in dev.
    if cfg!(debug_assertions) {
        return false;
    }

    // Check and modify temp.json
    let mut data: serde_json::Value = std::fs::read_to_string("temp.json")
        .ok()
        .and_then(|content| serde_json::from_str(&content).ok())
        .unwrap_or_else(|| {
            log().error("temp.json not found or invalid, creating a new one");
            serde_json::Value::Object(serde_json::Map::new())
        });

    if data
        .get("allow_multiple_instances")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        data["allow_multiple_instances"] = serde_json::Value::Bool(false);
        if let Err(e) = std::fs::write(
            "temp.json",
            serde_json::to_string_pretty(&data).unwrap_or_default(),
        ) {
            log().exception(&e, Some("Error writing to temp.json"), true, true, true);
        }
        return false;
    }

    // Python removes the current webdeck.exe once, then reports whether any
    // "webdeck" process remains — i.e. true when ≥2 match.
    count_webdeck_processes() >= 2
}

fn count_webdeck_processes() -> usize {
    process_names()
        .iter()
        .filter(|name| name.to_lowercase().contains("webdeck"))
        .count()
}

#[cfg(windows)]
fn process_names() -> Vec<String> {
    let mut names = Vec::new();
    if let Ok(output) = std::process::Command::new("tasklist")
        .args(["/FO", "CSV", "/NH"])
        .output()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            // First CSV field is the image name, e.g. "WebDeck.exe",...
            if let Some(first) = line.split("\",\"").next() {
                names.push(first.trim_matches('"').to_string());
            }
        }
    }
    names
}

#[cfg(target_os = "linux")]
fn process_names() -> Vec<String> {
    let mut names = Vec::new();
    if let Ok(entries) = std::fs::read_dir("/proc") {
        for entry in entries.flatten() {
            let comm = entry.path().join("comm");
            if let Ok(name) = std::fs::read_to_string(comm) {
                names.push(name.trim().to_string());
            }
        }
    }
    names
}

#[cfg(not(any(windows, target_os = "linux")))]
fn process_names() -> Vec<String> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_build_is_never_opened() {
        if cfg!(debug_assertions) {
            assert!(!is_opened());
        }
    }
}
