//! Port of `app/buttons/commands.py` — the `/send-data` command dispatcher.
//!
//! Branch structure and matching order are ported 1:1. OS actions map as:
//! - `subprocess.Popen(..., shell=True)` → [`spawn_shell`] (real, fire-and-forget).
//! - `pyautogui`/`keyboard`/`pyperclip`/`win32gui` → [`input`] helpers (via
//!   `enigo` input, `arboard` clipboard, `windows` crate window handles).
//! - `pycaw`/`comtypes` app-volume → [`app_volume`] (parsed 1:1, COM part via
//!   the `windows` crate).
//!
//! Linux desktop equivalents (systemd/logind/freedesktop/PipeWire) live in
//! [`desktop_linux`].

mod app_volume;
pub mod processes;
pub mod settings;
pub fn shutdown_children() {
    processes::shutdown();
    soundboard::mic::stop();
    soundboard::player::shutdown();
}
#[cfg(target_os = "linux")]
mod desktop_linux;
mod input;

use serde_json::{json, Value};

use crate::adapters::integrations::{exec, fetch, obs, spotify};
#[cfg(windows)]
use crate::app::utils::kill_nircmd::kill_nircmd;
use crate::app::utils::{
    firewall::fix_firewall_permission, logger::log, plugins::load_plugins::plugin_commands,
};

use self::app_volume::app_volume;
#[cfg(target_os = "linux")]
use self::desktop_linux::{
    linux_screensaver_settings, mpris_command, run_shell_checked, screensaver_dbus, shell_quote,
    tool_present,
};
use self::input::{clipboard_copy, hotkey, press_key, type_text};

/// Fire-and-forget shell spawn — port of `subprocess.Popen(..., shell=True)`.
pub(crate) fn spawn_shell(command: &str) {
    if let Err(e) = processes::spawn(command) {
        log().debug(&format!("Shell process failed to start: {e}"));
    }
}

fn success() -> Value {
    json!({"success": true})
}

fn failure(message: &str) -> Value {
    json!({"success": false, "message": message})
}

use crate::domain::command::{CommandKind, ParsedCommand};

pub fn execute(command: &ParsedCommand) -> Value {
    let command_arguments = command.original.clone();
    let message = command.normalized.clone();
    log().info(&format!("Command kind: {:?}", command.kind));
    match command.kind {
        CommandKind::Debug => {
            // Preserve the legacy debug action's success result, without logging payloads.
            let _ = serde_json::from_str::<Value>(&message.replacen("/debug-send", "", 1));
        }
        CommandKind::Exit => {
            crate::application::lifecycle::request_shutdown();
        }
        CommandKind::Usage => {
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
        }
        CommandKind::StopSound => {
            return soundboard::stopsound();
        }
        CommandKind::PlaySound => {
            let params = soundboard::get_params(&message);
            return soundboard::playsound(
                &params.file_path,
                params.sound_volume,
                params.ear_soundboard,
                params.localonly,
            );
        }
        CommandKind::Shutdown => {
            #[cfg(windows)]
            spawn_shell("shutdown /s /f /t 0");
            #[cfg(target_os = "linux")]
            spawn_shell("systemctl poweroff");
            #[cfg(not(any(windows, target_os = "linux")))]
            log().warning("/PCshutdown: only supported on Windows and Linux");
        }
        CommandKind::Reboot => {
            #[cfg(windows)]
            spawn_shell("shutdown /r /f /t 0");
            #[cfg(target_os = "linux")]
            spawn_shell("systemctl reboot");
            #[cfg(not(any(windows, target_os = "linux")))]
            log().warning("/PCrestart: only supported on Windows and Linux");
        }
        CommandKind::Sleep => {
            #[cfg(windows)]
            spawn_shell("rundll32.exe powrprof.dll,SetSuspendState 0,1,0");
            #[cfg(target_os = "linux")]
            spawn_shell("systemctl suspend");
            #[cfg(not(any(windows, target_os = "linux")))]
            log().warning("/PCsleep: only supported on Windows and Linux");
        }
        CommandKind::Hibernate => {
            #[cfg(windows)]
            spawn_shell("shutdown /h /t 0");
            #[cfg(target_os = "linux")]
            spawn_shell("systemctl hibernate");
            #[cfg(not(any(windows, target_os = "linux")))]
            log().warning("/PChibernate: only supported on Windows and Linux");
        }
        CommandKind::Lock => {
            #[cfg(windows)]
            spawn_shell("Rundll32.exe user32.dll,LockWorkStation");
            // logind locks every compositor; fall back to the ScreenSaver bus API.
            #[cfg(target_os = "linux")]
            if !(tool_present("loginctl") && run_shell_checked("loginctl lock-session")) {
                screensaver_dbus("Lock", "");
            }
            #[cfg(not(any(windows, target_os = "linux")))]
            log().warning("/locksession: only supported on Windows and Linux");
        }
        CommandKind::ScreensaverSettings => {
            #[cfg(windows)]
            spawn_shell("rundll32.exe desk.cpl,InstallScreenSaver toasters.scr");
            #[cfg(target_os = "linux")]
            linux_screensaver_settings();
            #[cfg(not(any(windows, target_os = "linux")))]
            log().warning("/screensaversettings: only supported on Windows and Linux");
        }
        CommandKind::Screensaver => {
            if message.ends_with("on")
                || message.ends_with("/screensaver")
                || message.ends_with("start")
            {
                #[cfg(windows)]
                spawn_shell("%windir%\\system32\\scrnsave.scr /s");
                #[cfg(target_os = "linux")]
                screensaver_dbus("SetActive", "true");
                #[cfg(not(any(windows, target_os = "linux")))]
                log().warning("/screensaver: only supported on Windows and Linux");
            } else if message.ends_with("hard")
                || message.ends_with("full")
                || message.ends_with("black")
            {
                #[cfg(windows)]
                {
                    spawn_shell("\"lib/nircmd.exe\" monitor off");
                    kill_nircmd();
                }
                // DPMS off: KDE's kscreen-doctor, else X11 xset.
                #[cfg(target_os = "linux")]
                if tool_present("kscreen-doctor") {
                    spawn_shell("kscreen-doctor --dpms off");
                } else if tool_present("xset") {
                    spawn_shell("xset dpms force off");
                } else {
                    log().warning("/screensaver hard: needs kscreen-doctor or xset");
                }
                #[cfg(not(any(windows, target_os = "linux")))]
                log().warning("/screensaver: only supported on Windows and Linux");
            } else if message.ends_with("off") || message.ends_with("false") {
                #[cfg(windows)]
                press_key("CTRL");
                // SimulateUserActivity wakes the locker without key injection
                // (which Wayland forbids); CTRL fallback needs an X server.
                #[cfg(target_os = "linux")]
                if !screensaver_dbus("SimulateUserActivity", "") {
                    press_key("CTRL");
                }
                #[cfg(not(any(windows, target_os = "linux")))]
                log().warning("/screensaver: only supported on Windows and Linux");
            }
        }
        CommandKind::Key => {
            let key = message.replacen("/key", "", 1);
            press_key(key.trim());
        }
        CommandKind::RestartDesktop => {
            #[cfg(windows)]
            {
                spawn_shell("taskkill /f /im explorer.exe");
                std::thread::sleep(std::time::Duration::from_millis(500));
                spawn_shell("explorer.exe");
                // NOTE: Python passes the HWND int to close() (TypeError → HTTP 500
                // upstream); close by title instead — same intent, no crash.
                if window::get_by_name("explorer.exe").is_ok() {
                    let _ = window::close("explorer.exe");
                }
            }
            // Closest Linux equivalent: restart the desktop shell (KDE).
            // GNOME on Wayland forbids shell restarts; other desktops vary.
            #[cfg(target_os = "linux")]
            {
                let desktop = std::env::var("XDG_CURRENT_DESKTOP")
                    .unwrap_or_default()
                    .to_lowercase();
                if desktop.contains("kde") {
                    spawn_shell("killall plasmashell; sleep 0.5; plasmashell");
                } else {
                    log().warning(&format!(
                    "/restartexplorer: desktop shell restart is only mapped for KDE (got {desktop:?})"
                ));
                }
            }
            #[cfg(not(any(windows, target_os = "linux")))]
            log().warning("/restartexplorer: only supported on Windows and Linux");
        }
        CommandKind::Kill => {
            let window_name = message
                .replace("/kill", "")
                .replace("/taskill", "")
                .replace("/taskkill", "")
                .replace("/forceclose", "");
            match window::get_by_name(&window_name) {
                Ok(hwnd) => {
                    log().debug(&format!("Window '{window_name}' found with handle: {hwnd}"))
                }
                Err(_) => log().debug(&format!("Window '{window_name}' not found")),
            }
            // NOTE: Python passes the HWND int to close() (TypeError → taskkill
            // fallback, and closes a random window when None); close by exact
            // title instead, keeping the taskkill/pkill fallback.
            if window::close(&window_name).is_err() {
                #[cfg(windows)]
                {
                    let with_exe = if window_name.contains('.') {
                        window_name.clone()
                    } else {
                        format!("{window_name}.exe")
                    };
                    spawn_shell(&format!("taskkill /f /im {with_exe}"));
                }
                // Exact process-name match (≈ taskkill /im semantics).
                #[cfg(target_os = "linux")]
                spawn_shell(&format!("pkill -x {}", shell_quote(window_name.trim())));
                #[cfg(not(any(windows, target_os = "linux")))]
                log().warning("/kill fallback: only supported on Windows and Linux");
            }
        }
        CommandKind::Restart => {
            #[cfg(windows)]
            {
                let mut exe = message.replace("/restart", "");
                if !exe.contains('.') {
                    exe.push_str(".exe");
                }
                spawn_shell(&format!("taskkill /f /im {exe}"));
                spawn_shell(&format!("start {exe}"));
            }
            #[cfg(target_os = "linux")]
            {
                let exe = message.replace("/restart", "");
                let exe = exe.trim();
                spawn_shell(&format!(
                    "pkill -x {q}; sleep 0.5; {q} &",
                    q = shell_quote(exe)
                ));
            }
            #[cfg(not(any(windows, target_os = "linux")))]
            log().warning("/restart: only supported on Windows and Linux");
        }
        CommandKind::ClearClipboard => {
            #[cfg(windows)]
            spawn_shell("cmd /c \"echo off | clip\"");
            // arboard clears the clipboard on every platform (wl-copy/xclip
            // backed on Linux); more reliable than shelling out.
            #[cfg(not(windows))]
            clipboard_copy("");
        }
        CommandKind::Write => {
            type_text(&message.replacen("/write ", "", 1));
        }
        CommandKind::WriteAndSend => {
            type_text(&message.replacen("/writeandsend ", "", 1));
            press_key("ENTER");
        }
        CommandKind::AppVolume => {
            app_volume(&message);
        }
        CommandKind::Mute => {
            #[cfg(windows)]
            press_key("volumemute");
            // PipeWire/PulseAudio toggle (no key injection needed).
            #[cfg(target_os = "linux")]
            if tool_present("pactl") {
                spawn_shell("pactl set-sink-mute @DEFAULT_SINK@ toggle");
            } else {
                press_key("volumemute");
            }
            #[cfg(not(any(windows, target_os = "linux")))]
            press_key("volumemute");
        }
        CommandKind::PlayPause => {
            #[cfg(windows)]
            press_key("playpause");
            #[cfg(target_os = "linux")]
            mpris_command("PlayPause");
            #[cfg(not(any(windows, target_os = "linux")))]
            press_key("playpause");
        }
        CommandKind::Previous => {
            #[cfg(windows)]
            press_key("prevtrack");
            #[cfg(target_os = "linux")]
            mpris_command("Previous");
            #[cfg(not(any(windows, target_os = "linux")))]
            press_key("prevtrack");
        }
        CommandKind::Next => {
            #[cfg(windows)]
            press_key("nexttrack");
            #[cfg(target_os = "linux")]
            mpris_command("Next");
            #[cfg(not(any(windows, target_os = "linux")))]
            press_key("nexttrack");
        }
        CommandKind::SpeechRecognition => {
            hotkey(&["win", "h"]);
        }
        CommandKind::CloseFocused => {
            if let Ok(hwnd) = window::get_focused() {
                let _ = window::close(&hwnd);
                #[cfg(windows)]
                {
                    spawn_shell(&format!("taskkill /f /im {hwnd}"));
                    spawn_shell(&format!("taskkill /f /im {hwnd}.exe"));
                }
                #[cfg(target_os = "linux")]
                spawn_shell(&format!("pkill -x {}", shell_quote(hwnd.trim())));
                #[cfg(not(any(windows, target_os = "linux")))]
                log().warning("/superAltF4 fallback: only supported on Windows and Linux");
            }
        }
        CommandKind::Foreground => {
            // FIXME (upstream): fix /firstplan
            let window_name = message.replace("/firstplan", "").trim().to_string();
            match window::get_by_name(&window_name) {
                Ok(hwnd) => {
                    window::foreground(hwnd);
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
        }
        CommandKind::Microphone => {
            audio::set_microphone_by_name(message.replace("/setmicrophone", "").trim());
        }
        CommandKind::Speakers => {
            audio::set_speakers_by_name(message.replace("/setoutputdevice", "").trim());
        }
        CommandKind::Copy => {
            if message.trim() == "/copy" {
                hotkey(&["ctrl", "c"]);
            } else {
                let mut msg = message.replacen("/copy ", "", 1);
                if msg.starts_with("/copy") {
                    msg = message.replace("/copy", "");
                }
                clipboard_copy(&msg);
            }
        }
        CommandKind::Paste => {
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
        }
        CommandKind::Cut => {
            hotkey(&["ctrl", "x"]);
        }
        CommandKind::Clipboard => {
            hotkey(&["win", "v"]);
        }
        CommandKind::Volume => {
            if let Err(message) = audio::change_volume(&message) {
                return failure(&message);
            }
        }
        CommandKind::Spotify => {
            return spotify::handle_command(&message);
        }
        CommandKind::Obs => {
            return obs::handle_command(&message);
        }
        CommandKind::ColorPicker => {
            color_picker::handle_command(&message);
        }
        CommandKind::Open => {
            system::handle_command(&message);
        }
        CommandKind::Exec => {
            if let Err(message) = exec::python(&message) {
                return failure(&message);
            }
        }
        CommandKind::Batch => {
            if let Err(message) = exec::batch(&message) {
                return failure(&message);
            }
        }
        CommandKind::Fetch => {
            // Raw message: arg boundaries are `<|§|>` (the form drops empty
            // values, so the normalized space-joined text is unparseable).
            return fetch::fetch(&command_arguments);
        }
        CommandKind::BypassFirewall => {
            fix_firewall_permission();
        }
        CommandKind::Plugin | CommandKind::Unknown => {
            // Plugin commands (port of the all_func loop; arity inspection is
            // replaced by the PluginFn(&[String]) adapter convention).
            for commands in plugin_commands().values() {
                for (command, func) in commands {
                    if message
                        .strip_prefix('/')
                        .unwrap_or(&message)
                        .starts_with(command)
                    {
                        let command_arguments =
                            command_arguments.replacen(&format!("/{command} "), "", 1);
                        let args: Vec<String> = command_arguments
                            .split("<|§|>")
                            .map(|s| s.to_string())
                            .collect();
                        func(&args);
                    }
                }
            }
        }
    }
    success()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::buttons::commands::handle_command;

    #[test]
    fn volume_errors_surface_as_failure() {
        // Pure: bad targets fail before any audio call; the route must
        // report {"success": false} like Python, never fake success.
        let failed = handle_command("/volume set abc");
        assert_eq!(failed.get("success"), Some(&json!(false)));
        assert!(failed.get("message").is_some());
        let ok = handle_command("/volume");
        assert_eq!(ok.get("success"), Some(&json!(true)));
    }
}

pub mod audio;
pub mod color_picker;
pub mod soundboard;
pub mod system;
pub mod usage;
pub mod window;
