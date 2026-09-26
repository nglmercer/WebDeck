//! Port of `app/buttons/commands.py` — the `/send-data` command dispatcher.
//!
//! Branch structure and matching order are ported 1:1. OS actions map as:
//! - `subprocess.Popen(..., shell=True)` → [`spawn_shell`] (real, fire-and-forget).
//! - `pyautogui`/`keyboard`/`pyperclip`/`win32gui` → helpers below, TODO via
//!   `enigo` (input), `arboard` (clipboard), `windows` crate (window handles).
//! - `pycaw`/`comtypes` app-volume → parsed 1:1, COM part TODO (`windows` crate).

use serde_json::{json, Value};

use crate::app::buttons::{audio, color_picker, exec, obs, soundboard, spotify, system, usage, window};
use crate::app::utils::{
    firewall::fix_firewall_permission, kill_nircmd::kill_nircmd, logger::log,
    plugins::load_plugins::plugin_commands,
};

/// Fire-and-forget shell spawn — port of `subprocess.Popen(..., shell=True)`.
fn spawn_shell(command: &str) {
    #[cfg(windows)]
    let spawned = std::process::Command::new("cmd")
        .args(["/C", command])
        .spawn();
    #[cfg(not(windows))]
    let spawned = std::process::Command::new("sh").args(["-c", command]).spawn();
    if let Err(e) = spawned {
        log().debug(&format!("spawn_shell({command:?}) failed: {e}"));
    }
}

// --- Input/clipboard backends (TODO: enigo / arboard) ------------------------

fn press_key(key: &str) {
    log().warning(&format!("press_key({key:?}): keyboard backend (enigo) not ported yet"));
}

fn hotkey(keys: &[&str]) {
    log().warning(&format!("hotkey({keys:?}): keyboard backend (enigo) not ported yet"));
}

fn type_text(text: &str) {
    log().warning(&format!(
        "type_text({} chars): keyboard backend (enigo) not ported yet",
        text.len()
    ));
}

fn clipboard_copy(text: &str) {
    log().warning(&format!(
        "clipboard_copy({} chars): clipboard backend (arboard) not ported yet",
        text.len()
    ));
}

fn success() -> Value {
    json!({"success": true})
}

fn failure(message: &str) -> Value {
    json!({"success": false, "message": message})
}

/// Port of `handle_command`.
pub fn handle_command(message: &str) -> Value {
    let command_arguments = message.to_string();
    let message = message.replace("<|§|>", " ");

    if message == "/bypass-windows-firewall" {
        fix_firewall_permission();
    }

    if !message
        .trim()
        .replace('\n', "")
        .replace('\r', "")
        .is_empty()
    {
        log().info(&format!("Command received: {message}"));
    }

    if message.starts_with("/debug-send") {
        let payload = message.replace("/debug-send", "").replace('\'', "\"");
        match serde_json::from_str::<Value>(&payload) {
            Ok(data) => log().debug(&format!("debug-send payload: {data}")),
            Err(e) => log().debug(&format!("debug-send parse error: {e}")),
        }
    } else if message.starts_with("/exit") {
        std::process::exit(0);
    } else if message.starts_with("/usage") {
        // NOTE: Python marks this branch "useless btw" (the /usage route
        // serves this); kept 1:1 anyway.
        let mut asked_device: Vec<Vec<String>> = Vec::new();
        let device = usage::extract_asked_device(&message);
        if !device.is_empty() {
            asked_device.push(device);
        }
        log().debug(&format!("Asked device: {asked_device:?}"));
        let data = usage::get_usage(Some(false), &asked_device);
        log().debug(&format!("Usage data: {data}"));
        return data;
    } else if message.starts_with("/stop_sound") {
        return soundboard::stopsound();
    } else if message.starts_with("/playsound ") || message.starts_with("/playlocalsound ") {
        let params = soundboard::get_params(&message);
        return soundboard::playsound(
            &params.file_path,
            params.sound_volume,
            params.ear_soundboard,
            params.localonly,
        );
    } else if message.starts_with("/PCshutdown") {
        spawn_shell("shutdown /s /f /t 0");
    } else if message.starts_with("/PCrestart") {
        spawn_shell("shutdown /r /f /t 0");
    } else if message.starts_with("/PCsleep") {
        spawn_shell("rundll32.exe powrprof.dll,SetSuspendState 0,1,0");
    } else if message.starts_with("/PChibernate") {
        spawn_shell("shutdown /h /t 0");
    } else if message.starts_with("/locksession") {
        spawn_shell("Rundll32.exe user32.dll,LockWorkStation");
    } else if message.starts_with("/screensaversettings") {
        spawn_shell("rundll32.exe desk.cpl,InstallScreenSaver toasters.scr");
    } else if message.starts_with("/screensaver") && !message.starts_with("/screensaversettings") {
        if message.ends_with("on") || message.ends_with("/screensaver") || message.ends_with("start")
        {
            spawn_shell("%windir%\\system32\\scrnsave.scr /s");
        } else if message.ends_with("hard") || message.ends_with("full") || message.ends_with("black")
        {
            spawn_shell("\"lib/nircmd.exe\" monitor off");
            kill_nircmd();
        } else if message.ends_with("off") || message.ends_with("false") {
            press_key("CTRL");
        }
    } else if message.starts_with("/key") {
        let key = message.replacen("/key", "", 1);
        press_key(key.trim());
    } else if message.starts_with("/restartexplorer") {
        spawn_shell("taskkill /f /im explorer.exe");
        std::thread::sleep(std::time::Duration::from_millis(500));
        spawn_shell("explorer.exe");
        if let Ok(hwnd) = window::get_by_name("explorer.exe") {
            let _ = window::close(&hwnd);
        }
    } else if message.starts_with("/kill")
        || message.starts_with("/taskill")
        || message.starts_with("/taskkill")
        || message.starts_with("/forceclose")
    {
        let mut window_name = message
            .replace("/kill", "")
            .replace("/taskill", "")
            .replace("/taskkill", "")
            .replace("/forceclose", "");
        match window::get_by_name(&window_name) {
            Ok(hwnd) => log().debug(&format!("Window '{window_name}' found with handle: {hwnd}")),
            Err(_) => log().debug(&format!("Window '{window_name}' not found")),
        }
        if window::close(&window_name).is_err() {
            if !window_name.contains('.') {
                window_name.push_str(".exe");
            }
            spawn_shell(&format!("taskkill /f /im {window_name}"));
        }
    } else if message.starts_with("/restart") {
        let mut exe = message.replace("/restart", "");
        if !exe.contains('.') {
            exe.push_str(".exe");
        }
        spawn_shell(&format!("taskkill /f /im {exe}"));
        spawn_shell(&format!("start {exe}"));
    } else if message.starts_with("/clearclipboard") {
        spawn_shell("cmd /c \"echo off | clip\"");
    } else if message.starts_with("/write ") {
        type_text(&message.replacen("/write ", "", 1));
    } else if message.starts_with("/writeandsend ") {
        type_text(&message.replacen("/writeandsend ", "", 1));
        press_key("ENTER");
    } else if message.starts_with("/appvolume +")
        || message.starts_with("/appvolume -")
        || message.starts_with("/appvolume set")
    {
        // TODO(port): per-app volume via `windows` crate CoreAudio (pycaw
        // equivalent). Argument parsing is ported 1:1 below.
        let command = message
            .replacen("/appvolume ", "", 1)
            .replace("set ", "set");
        let parts: Vec<&str> = command.split_whitespace().collect();
        log().warning(&format!(
            "appvolume {parts:?}: CoreAudio backend not ported yet"
        ));
    } else if message.starts_with("/soundcontrol mute") {
        press_key("volumemute");
    } else if message.starts_with("/mediacontrol playpause") {
        press_key("playpause");
    } else if message.starts_with("/mediacontrol previous") {
        press_key("prevtrack");
    } else if message.starts_with("/mediacontrol next") {
        press_key("nexttrack");
    } else if message.starts_with("/speechrecognition") {
        hotkey(&["win", "h"]);
    } else if message.starts_with("/superAltF4") {
        if let Ok(hwnd) = window::get_focused() {
            let _ = window::close(&hwnd);
            spawn_shell(&format!("taskkill /f /im {hwnd}"));
            spawn_shell(&format!("taskkill /f /im {hwnd}.exe"));
        }
    } else if message.starts_with("/firstplan") {
        // FIXME (upstream): fix /firstplan
        let window_name = message.replace("/firstplan", "").trim().to_string();
        match window::get_by_name(&window_name) {
            Ok(_) => {
                window::bring_window_to_front(&window_name);
                press_key("ENTER");
                log().success(&format!(
                    "Window '{window_name}' has been brought to the foreground"
                ));
            }
            Err(_) => {
                log().error(&format!("Window '{window_name}' not found"));
                return failure(&format!("Window '{window_name}' not found"));
            }
        }
    } else if message.starts_with("/setmicrophone") {
        audio::set_microphone_by_name(message.replace("/setmicrophone", "").trim());
    } else if message.starts_with("/setoutputdevice") {
        audio::set_speakers_by_name(message.replace("/setoutputdevice", "").trim());
    } else if message.starts_with("/copy") {
        if message.trim() == "/copy" {
            hotkey(&["ctrl", "c"]);
        } else {
            let mut msg = message.replacen("/copy ", "", 1);
            if msg.starts_with("/copy") {
                msg = message.replace("/copy", "");
            }
            clipboard_copy(&msg);
        }
    } else if message.starts_with("/paste") {
        if message.trim() == "/paste" {
            hotkey(&["ctrl", "v"]);
        } else {
            let mut msg = message.replacen("/paste ", "", 1);
            if msg.starts_with("/paste") {
                msg = message.replace("/paste", "");
            }
            clipboard_copy(&msg);
            hotkey(&["ctrl", "v"]);
        }
    } else if message.starts_with("/cut") {
        hotkey(&["ctrl", "x"]);
    } else if message.starts_with("/clipboard") {
        hotkey(&["win", "v"]);
    } else if message.starts_with("/volume") {
        audio::change_volume(&message);
    } else if message.starts_with("/spotify") {
        return spotify::handle_command(&message);
    } else if message.starts_with("/obs") {
        return obs::handle_command(&message);
    } else if message.starts_with("/colorpicker") {
        color_picker::handle_command(&message);
    } else if message.starts_with("/openfolder")
        || message.starts_with("/opendir")
        || message.starts_with("/openfile")
        || message.starts_with("/start")
    {
        system::handle_command(&message);
    } else if message.starts_with("/exec") {
        exec::python(&message);
    } else if message.starts_with("/batch") {
        exec::batch(&message);
    } else {
        // Plugin commands (port of the all_func loop; arity inspection is
        // replaced by the PluginFn(&[String]) adapter convention).
        for commands in plugin_commands().values() {
            for (command, func) in commands {
                if message
                    .strip_prefix('/')
                    .unwrap_or(&message)
                    .starts_with(command)
                {
                    let command_arguments = command_arguments
                        .replacen(&format!("/{command} "), "", 1);
                    let args: Vec<String> = command_arguments
                        .split("<|§|>")
                        .map(|s| s.to_string())
                        .collect();
                    func(&args);
                }
            }
        }
    }

    success()
}
