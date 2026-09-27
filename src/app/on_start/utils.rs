//! Port of `app/on_start/utils.py`.

use serde_json::Value;

use crate::app::updater::{check::check_for_updates, updater::check_files};
use crate::app::utils::{
    args::get_args,
    get_local_ip::get_local_ip,
    global_variables::set_global_variable,
    logger::log,
    plugins::load_plugins::load_plugins,
    settings::{get_config::get_config, save_config::save_config},
};

/// Port of `color_distance`.
pub fn color_distance(color1: &str, color2: &str) -> f64 {
    let channel =
        |color: &str, i: usize| i32::from_str_radix(&color[i + 1..i + 3], 16).unwrap_or(0);
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

/// Create a `WebDeck.lnk` shortcut in a known folder (port of the
/// `WScript.Shell` blocks, via `IShellLinkW` + `IPersistFile` — same
/// shortcut, no scripting host involved).
#[cfg(windows)]
fn create_shortcut_in_known_folder(folder_id: &windows::core::GUID) {
    use windows::core::{Interface as _, HSTRING};
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_ALL, COINIT_APARTMENTTHREADED,
    };
    use windows::Win32::UI::Shell::{
        IShellLinkW, SHGetKnownFolderPath, ShellLink, KF_FLAG_DEFAULT,
    };

    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
    }
    let result = (|| -> windows::core::Result<()> {
        unsafe {
            let link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_ALL)?;
            let exe = std::env::current_exe().map_err(|e| {
                windows::core::Error::new(windows::core::HRESULT(-1), e.to_string())
            })?;
            link.SetPath(&HSTRING::from(exe.to_string_lossy().as_ref()))?;
            if let Some(dir) = exe.parent() {
                link.SetWorkingDirectory(&HSTRING::from(dir.to_string_lossy().as_ref()))?;
            }
            let folder = SHGetKnownFolderPath(folder_id, KF_FLAG_DEFAULT, None)?;
            let raw = folder.0;
            let mut length = 0;
            while *raw.add(length) != 0 {
                length += 1;
            }
            let folder = String::from_utf16_lossy(std::slice::from_raw_parts(raw, length));
            windows::Win32::System::Com::CoTaskMemFree(Some(raw as *const _));
            let persist: windows::Win32::System::Com::IPersistFile = link.cast()?;
            persist.Save(&HSTRING::from(format!("{folder}\\WebDeck.lnk")), true)?;
            Ok(())
        }
    })();
    unsafe {
        CoUninitialize();
    }
    if let Err(e) = result {
        log().exception(&e, Some("Failed to create shortcut"), true, true, true);
    }
}

/// Create `%APPDATA%\Microsoft\Windows\Start Menu\Programs\WebDeck.lnk`
/// (port of the `windows_start_menu_shortcut` block in `on_start`).
#[cfg(windows)]
fn create_start_menu_shortcut() {
    use windows::Win32::UI::Shell::FOLDERID_Programs;
    create_shortcut_in_known_folder(&FOLDERID_Programs);
}

/// Create the Startup-folder shortcut (port of the `windows_startup` block
/// in `saveconfig`).
#[cfg(windows)]
pub fn create_startup_shortcut() {
    use windows::Win32::UI::Shell::FOLDERID_Startup;
    create_shortcut_in_known_folder(&FOLDERID_Startup);
}

/// Linux equivalent of the `.lnk` shortcuts: a freedesktop `.desktop` file
/// (`~/.local/share/applications` for the app menu,
/// `~/.config/autostart` for startup). Only created when missing (like
/// Python's `not os.path.exists` gate) and only for release builds
/// (`frozen` equivalent — dev runs must not litter the menu).
#[cfg(target_os = "linux")]
fn write_desktop_file(relative_dir: &str) -> Result<(), String> {
    let home = std::env::var("HOME").map_err(|_| "HOME is not set".to_string())?;
    let dir = format!("{home}/{relative_dir}");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = format!("{dir}/WebDeck.desktop");
    if std::path::Path::new(&path).exists() {
        return Ok(());
    }
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let workdir = exe
        .parent()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();
    std::fs::write(&path, desktop_entry(&exe.to_string_lossy(), &workdir))
        .map_err(|e| e.to_string())?;
    log().debug(&format!("Created Linux shortcut: {path}"));
    Ok(())
}

/// The `.desktop` file content (pure, unit-tested).
#[cfg(target_os = "linux")]
fn desktop_entry(exec: &str, workdir: &str) -> String {
    format!(
        "[Desktop Entry]\nType=Application\nName=WebDeck\nExec={exec}\nPath={workdir}\n\
         Icon={workdir}/static/icons/icon.ico\nTerminal=false\nCategories=Utility;\n"
    )
}

/// Remove a previously created `.desktop` file (toggle-off path).
#[cfg(target_os = "linux")]
fn remove_desktop_file(relative_dir: &str) {
    if let Ok(home) = std::env::var("HOME") {
        let path = format!("{home}/{relative_dir}/WebDeck.desktop");
        if std::path::Path::new(&path).exists() {
            let _ = std::fs::remove_file(&path);
        }
    }
}

/// Create `~/.local/share/applications/WebDeck.desktop` (port of the
/// `windows_start_menu_shortcut` block in `on_start`).
#[cfg(target_os = "linux")]
fn create_start_menu_shortcut() {
    if let Err(e) = write_desktop_file(".local/share/applications") {
        log().exception(&e, Some("Failed to create shortcut"), true, true, true);
    }
}

/// Create `~/.config/autostart/WebDeck.desktop` (port of the
/// `windows_startup` block in `saveconfig`).
#[cfg(target_os = "linux")]
pub fn create_startup_shortcut() {
    if let Err(e) = write_desktop_file(".config/autostart") {
        log().exception(&e, Some("Failed to create shortcut"), true, true, true);
    }
}

/// Remove `~/.config/autostart/WebDeck.desktop` (toggle-off path).
#[cfg(target_os = "linux")]
pub fn remove_startup_shortcut() {
    remove_desktop_file(".config/autostart");
}

/// Port of `get_gpu_method`.
///
/// Keeps the config default + save flow 1:1; the NVML probe is
/// `nvml_wrapper::Nvml::init()` (failure → `"AMD"` fallback).
///
/// Deviation: a stuck `"None"` method is re-probed (NVML → amdgpu →
/// stay `"None"`). Past NVML failures persisted `"None"` into the config
/// with no recovery path, bricking GPU tiles on machines whose GPU works
/// fine under another method.
pub fn get_gpu_method() -> Value {
    use crate::app::buttons::usage::gpu_amd::has_amdgpu_card;

    fn method_of(config: &Value) -> &str {
        config
            .get("settings")
            .and_then(|s| s.get("gpu_method"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
    }
    fn set_method(config: &mut Value, method: &str) {
        if let Some(settings) = config.get_mut("settings").and_then(|s| s.as_object_mut()) {
            settings.insert(
                "gpu_method".to_string(),
                Value::String(method.to_string()),
            );
        }
    }

    let mut config = get_config(false, false);
    if method_of(&config).is_empty() {
        set_method(&mut config, "nvidia (NVML)");
    }
    if matches!(method_of(&config), "nvidia (NVML)" | "nvidia (pynvml)")
        && nvml_wrapper::Nvml::init().is_err()
    {
        set_method(&mut config, "AMD");
    } else if method_of(&config) == "None" {
        if nvml_wrapper::Nvml::init().is_ok() {
            set_method(&mut config, "nvidia (NVML)");
        } else if has_amdgpu_card() {
            set_method(&mut config, "AMD");
        }
        if method_of(&config) != "None" {
            log().debug(&format!(
                "Recovered stuck gpu_method to {:?}",
                method_of(&config)
            ));
        }
    }
    save_config(config.clone());
    config
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
            create_start_menu_shortcut();
        } else {
            // NOTE (upstream): reserved for future portable-version use.
        }
    }
    #[cfg(target_os = "linux")]
    {
        let wants_shortcut = config
            .get("settings")
            .and_then(|s| s.get("windows_start_menu_shortcut"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        // `frozen` equivalent: release builds only.
        if wants_shortcut && !cfg!(debug_assertions) {
            create_start_menu_shortcut();
        }
    }
    #[cfg(not(any(windows, target_os = "linux")))]
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
        Value::Array(loaded_plugins.into_iter().map(Value::String).collect()),
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
    // NOTE: Python's `fix_vlc_cache` step is gone: sound plays through
    // rodio/cpal now, so no VLC install (and no plugin-cache rebuild)
    // is needed on any platform.
    tokio::spawn(async move {
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
    fn stuck_none_method_recovers_to_working_backend() {
        use crate::app::buttons::usage::gpu_amd::has_amdgpu_card;
        use crate::app::utils::settings::get_config::test_support::{
            config_guard, seed_config,
        };
        let _guard = config_guard();
        seed_config(&serde_json::json!({
            "url": {"port": 5000},
            "front": {"buttons": {}},
            "settings": {"gpu_method": "None"},
        }));
        let config = get_gpu_method();
        let method = config["settings"]["gpu_method"].as_str().unwrap_or("");
        if nvml_wrapper::Nvml::init().is_ok() {
            assert_eq!(method, "nvidia (NVML)");
        } else if has_amdgpu_card() {
            assert_eq!(method, "AMD");
        } else {
            assert_eq!(method, "None");
        }
    }

    #[test]
    fn color_distance_black_white() {
        let d = color_distance("#000000", "#ffffff");
        assert!((d - (3.0f64 * 255.0 * 255.0).sqrt()).abs() < 1e-6);
        assert_eq!(color_distance("#123456", "#123456"), 0.0);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn desktop_entry_has_required_keys() {
        let entry = desktop_entry("/opt/webdeck/webdeck", "/opt/webdeck");
        assert!(entry.starts_with("[Desktop Entry]\n"));
        assert!(entry.contains("\nExec=/opt/webdeck/webdeck\n"));
        assert!(entry.contains("\nPath=/opt/webdeck\n"));
        assert!(entry.contains("\nType=Application\n"));
        assert!(entry.contains("Terminal=false"));
    }
}
