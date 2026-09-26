//! Port of `app/on_start/utils.py`.

use serde_json::Value;

use crate::app::updater::{check::check_for_updates, updater::check_files};
use crate::app::utils::{
    args::get_args, get_local_ip::get_local_ip, global_variables::set_global_variable,
    logger::log, plugins::load_plugins::load_plugins,
    settings::{get_config::get_config, save_config::save_config},
};

/// Port of `color_distance`.
pub fn color_distance(color1: &str, color2: &str) -> f64 {
    let channel = |color: &str, i: usize| {
        i32::from_str_radix(&color[i + 1..i + 3], 16).unwrap_or(0)
    };
    let (r1, g1, b1) = (channel(color1, 0), channel(color1, 2), channel(color1, 4));
    let (r2, g2, b2) = (channel(color2, 0), channel(color2, 2), channel(color2, 4));
    (((r1 - r2).pow(2) + (g1 - g2).pow(2) + (b1 - b2).pow(2)) as f64).sqrt()
}

/// Port of `sort_colorsjson`.
///
/// Same nearest-neighbor ordering; the Gist fallback uses `reqwest`
/// (async — hence `on_start` is async in Rust).
pub async fn sort_colorsjson() {
    let mut data: Value = match std::fs::read_to_string("webdeck/colors.json") {
        Ok(content) => match serde_json::from_str(&content) {
            Ok(data) => data,
            Err(e) => {
                log().exception(&e, Some("Failed to load colors.json"), true, true, true);
                match fetch_colors_fallback().await {
                    Some(data) => data,
                    None => return,
                }
            }
        },
        Err(e) => {
            log().exception(&e, Some("Failed to load colors.json"), true, true, true);
            match fetch_colors_fallback().await {
                Some(data) => data,
                None => return,
            }
        }
    };

    let empty = Vec::new();
    let mut remaining = data.as_array().cloned().unwrap_or(empty);
    if remaining.is_empty() {
        return;
    }
    let mut sorted = vec![remaining.remove(0)];
    while !remaining.is_empty() {
        let current = sorted
            .last()
            .and_then(|c| c.get("hex_code"))
            .and_then(|v| v.as_str())
            .unwrap_or("#000000")
            .to_string();
        let mut best = 0;
        let mut best_distance = f64::INFINITY;
        for (i, candidate) in remaining.iter().enumerate() {
            let code = candidate
                .get("hex_code")
                .and_then(|v| v.as_str())
                .unwrap_or("#000000");
            let distance = color_distance(&current, code);
            if distance < best_distance {
                best_distance = distance;
                best = i;
            }
        }
        sorted.push(remaining.remove(best));
    }

    data = Value::Array(sorted);
    if std::fs::write(
        "webdeck/colors.json",
        serde_json::to_string_pretty(&data).unwrap_or_default(),
    )
    .is_err()
    {
        log().error("Failed to write sorted webdeck/colors.json");
    }
}

async fn fetch_colors_fallback() -> Option<Value> {
    const URL: &str = "https://gist.githubusercontent.com/Lenochxd/12a1927943a2ce151560e1b9585d4bfa/raw/41d5a0dc9336827cefb217c1728f0e9415b1c7b9/colors_db.json";
    let response = reqwest::get(URL).await.ok()?;
    let data: Value = response.json().await.ok()?;
    if std::fs::write(
        "webdeck/colors.json",
        serde_json::to_string_pretty(&data).unwrap_or_default(),
    )
    .is_err()
    {
        log().exception(
            &"write failed",
            Some("Failed to load colors.json from Gist"),
            true,
            true,
            true,
        );
        return None;
    }
    Some(data)
}

/// Port of `get_gpu_method`.
///
/// Keeps the config default + save flow 1:1; the `pynvml` probe is TODO
/// (`nvml-wrapper`), so non-NVIDIA fallback detection is deferred.
pub fn get_gpu_method() -> Value {
    let mut config = get_config(false, false);
    if config
        .get("settings")
        .and_then(|s| s.get("gpu_method"))
        .is_none()
    {
        if let Some(settings) = config.get_mut("settings").and_then(|s| s.as_object_mut()) {
            settings.insert(
                "gpu_method".to_string(),
                Value::String("nvidia (pynvml)".to_string()),
            );
        }
    }
    // TODO(port): nvml prelude probe → fall back to "AMD" on NVMLError.
    save_config(config.clone());
    config
}

/// Port of `fix_vlc_cache`.
///
/// Non-Windows returns immediately like Python; the registry lookup +
/// `vlc-cache-gen` run is TODO via the `windows` crate (registry).
pub fn fix_vlc_cache() {
    if !cfg!(windows) {
        return;
    }
    // TODO(port): read HKLM\SOFTWARE\VideoLAN\VLC InstallDir and run
    // vlc-cache-gen.exe over the plugins dir.
    log().debug("fix_vlc_cache: registry backend not ported yet");
}

/// Port of `on_start` — returns `(config, commands, local_ip)`.
///
/// Async only because the colors.json Gist fallback needs `reqwest`.
pub async fn on_start() -> (Value, Value, String) {
    let mut config = get_config(true, true);

    #[cfg(windows)]
    {
        let wants_shortcut = config
            .get("settings")
            .and_then(|s| s.get("windows_start_menu_shortcut"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        if wants_shortcut {
            // TODO(port): Start Menu shortcut via windows crate (WScript.Shell).
            log().debug("windows_start_menu_shortcut: shortcut creation not ported yet");
        } else {
            // NOTE (upstream): reserved for future portable-version use.
        }
    }
    #[cfg(not(windows))]
    let _ = &config;

    for dir in [".config/user_uploads", ".config/themes", ".config/plugins"] {
        if !std::path::Path::new(dir).exists() {
            let _ = std::fs::create_dir_all(dir);
        }
    }

    // Move legacy static/files/uploaded content to .config/user_uploads.
    if std::path::Path::new("static/files/uploaded").exists() {
        if let Ok(entries) = std::fs::read_dir("static/files/uploaded") {
            for entry in entries.flatten() {
                let src = entry.path();
                if let Some(name) = src.file_name() {
                    let dst = std::path::Path::new(".config/user_uploads").join(name);
                    let _ = std::fs::rename(&src, &dst);
                }
            }
        }
        let _ = std::fs::remove_dir_all("static/files/uploaded");
    }

    // Update new files.
    check_files();

    // Load config & get gpu method.
    config = get_gpu_method();

    // Check for updates.
    let args = get_args();
    let auto_updates = config
        .get("settings")
        .and_then(|s| s.get("auto_updates"))
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    if (auto_updates || args.force_update) && !args.no_auto_update {
        check_for_updates().await;
    }

    // Load commands.
    let commands = std::fs::read_to_string("webdeck/commands.json")
        .ok()
        .and_then(|content| serde_json::from_str(&content).ok())
        .unwrap_or(Value::Object(serde_json::Map::new()));
    let (commands, loaded_plugins) = load_plugins(commands);
    // Python stores the {plugin: {cmd: func}} map; callables live in the
    // static registry in Rust, so the global holds the loaded names.
    set_global_variable(
        "all_func",
        Value::Array(
            loaded_plugins.into_iter().map(Value::String).collect(),
        ),
    );

    // Get local ip.
    let local_ip = get_local_ip().unwrap_or_else(|_| "127.0.0.1".to_string());
    if config
        .get("url")
        .and_then(|u| u.get("ip"))
        .and_then(|v| v.as_str())
        == Some("local_ip")
    {
        if let Some(url) = config.get_mut("url").and_then(|u| u.as_object_mut()) {
            url.insert("ip".to_string(), Value::String(local_ip.clone()));
        }
        save_config(config.clone());
    }

    // Run threaded tasks.
    on_start_threaded(config.clone());

    (config, commands, local_ip)
}

/// Port of `on_start_threaded` — spawns a background task like Python's thread.
pub fn on_start_threaded(config: Value) {
    tokio::spawn(async move {
        fix_vlc_cache();

        let sort = config
            .get("settings")
            .and_then(|s| s.get("sort_colors_on_startup"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        if sort {
            sort_colorsjson().await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_distance_black_white() {
        let d = color_distance("#000000", "#ffffff");
        assert!((d - (3.0f64 * 255.0 * 255.0).sqrt()).abs() < 1e-6);
        assert_eq!(color_distance("#123456", "#123456"), 0.0);
    }
}
