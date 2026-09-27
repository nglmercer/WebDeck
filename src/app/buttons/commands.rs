//! Port of `app/buttons/commands.py` — the `/send-data` command dispatcher.
//!
//! Branch structure and matching order are ported 1:1. OS actions map as:
//! - `subprocess.Popen(..., shell=True)` → [`spawn_shell`] (real, fire-and-forget).
//! - `pyautogui`/`keyboard`/`pyperclip`/`win32gui` → helpers below, TODO via
//!   `enigo` (input), `arboard` (clipboard), `windows` crate (window handles).
//! - `pycaw`/`comtypes` app-volume → parsed 1:1, COM part TODO (`windows` crate).

use serde_json::{json, Value};

use crate::app::buttons::{
    audio, color_picker, exec, obs, soundboard, spotify, system, usage, window,
};
#[cfg(windows)]
use crate::app::utils::kill_nircmd::kill_nircmd;
use crate::app::utils::{
    firewall::fix_firewall_permission, logger::log, plugins::load_plugins::plugin_commands,
};

/// Fire-and-forget shell spawn — port of `subprocess.Popen(..., shell=True)`.
fn spawn_shell(command: &str) {
    #[cfg(windows)]
    let spawned = std::process::Command::new("cmd")
        .args(["/C", command])
        .spawn();
    #[cfg(not(windows))]
    let spawned = std::process::Command::new("sh")
        .args(["-c", command])
        .spawn();
    if let Err(e) = spawned {
        log().debug(&format!("spawn_shell({command:?}) failed: {e}"));
    }
}

// --- Linux desktop helpers ---------------------------------------------------
// The PC-control commands are `shutdown`/`rundll32`/`taskkill` one-liners on
// Windows; on Linux they map to the systemd/logind/freedesktop/PipeWire
// equivalents below. Every helper degrades to a log line when its tool is
// missing (Wayland and minimal installs vary).

/// True when a CLI tool exists on PATH.
#[cfg(target_os = "linux")]
fn tool_present(tool: &str) -> bool {
    std::process::Command::new("sh")
        .args(["-c", &format!("command -v {tool} >/dev/null 2>&1")])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Run a shell command, returning exit success (for fallback chains —
/// unlike fire-and-forget `spawn_shell`).
#[cfg(target_os = "linux")]
fn run_shell_checked(command: &str) -> bool {
    std::process::Command::new("sh")
        .args(["-c", command])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Shell-quote one argument (single-quote wrapping).
#[cfg(target_os = "linux")]
fn shell_quote(arg: &str) -> String {
    format!("'{}'", arg.replace('\'', "'\\''"))
}

/// Call `org.freedesktop.ScreenSaver.<method>` (lock/activate/idle-reset),
/// trying qdbus6 → qdbus → gdbus.
#[cfg(target_os = "linux")]
fn screensaver_dbus(method: &str, extra_args: &str) -> bool {
    const DEST: &str = "org.freedesktop.ScreenSaver";
    const PATH: &str = "/ScreenSaver";
    if tool_present("qdbus6") || tool_present("qdbus") {
        let qdbus = if tool_present("qdbus6") {
            "qdbus6"
        } else {
            "qdbus"
        };
        return run_shell_checked(&format!(
            "{qdbus} {DEST} {PATH} {DEST}.{method} {extra_args}"
        ));
    }
    if tool_present("gdbus") {
        return run_shell_checked(&format!(
            "gdbus call --session --dest {DEST} --object-path {PATH} --method {DEST}.{method} {extra_args}"
        ));
    }
    log().warning("screensaver_dbus: no qdbus/qdbus6/gdbus on PATH");
    false
}

/// Send an MPRIS `Player.<command>` (`PlayPause`/`Previous`/`Next`) to the
/// first registered media player (port of the media-key presses).
#[cfg(target_os = "linux")]
fn mpris_command(command: &str) {
    // Player discovery via the session bus service list.
    let lister = if tool_present("qdbus6") {
        Some("qdbus6")
    } else if tool_present("qdbus") {
        Some("qdbus")
    } else {
        None
    };
    let mut players: Vec<String> = Vec::new();
    if let Some(lister) = lister {
        if let Ok(output) = std::process::Command::new(lister).output() {
            players = String::from_utf8_lossy(&output.stdout)
                .lines()
                .filter(|l| l.starts_with("org.mpris.MediaPlayer2."))
                .map(|l| l.trim().to_string())
                .collect();
        }
    }
    if players.is_empty() && tool_present("gdbus") {
        if let Ok(output) = std::process::Command::new("gdbus")
            .args([
                "call",
                "--session",
                "--dest",
                "org.freedesktop.DBus",
                "--object-path",
                "/",
                "--method",
                "org.freedesktop.DBus.ListNames",
            ])
            .output()
        {
            // Output shape: ([':1.1', 'org.mpris.MediaPlayer2.foo', ...],)
            players = String::from_utf8_lossy(&output.stdout)
                .split('\'')
                .filter(|s| s.starts_with("org.mpris.MediaPlayer2."))
                .map(|s| s.to_string())
                .collect();
        }
    }
    let Some(player) = players.into_iter().next() else {
        log().warning(&format!(
            "mpris {command}: no media player on the session bus"
        ));
        return;
    };
    let sender = if tool_present("qdbus6") {
        format!("qdbus6 {player} /org/mpris/MediaPlayer2 org.mpris.MediaPlayer2.Player.{command}")
    } else if tool_present("qdbus") {
        format!("qdbus {player} /org/mpris/MediaPlayer2 org.mpris.MediaPlayer2.Player.{command}")
    } else {
        format!(
            "gdbus call --session --dest {player} --object-path /org/mpris/MediaPlayer2 \
             --method org.mpris.MediaPlayer2.Player.{command}"
        )
    };
    if !run_shell_checked(&sender) {
        log().warning(&format!("mpris {command}: send to {player} failed"));
    }
}

/// Open the desktop's screen-locker settings (KDE/GNOME best effort).
#[cfg(target_os = "linux")]
fn linux_screensaver_settings() {
    let desktop = std::env::var("XDG_CURRENT_DESKTOP")
        .unwrap_or_default()
        .to_lowercase();
    if desktop.contains("kde") && tool_present("systemsettings") {
        spawn_shell("systemsettings kcm_screenlocker");
    } else if desktop.contains("gnome") && tool_present("gnome-control-center") {
        spawn_shell("gnome-control-center screen");
    } else if tool_present("systemsettings") {
        spawn_shell("systemsettings kcm_screenlocker");
    } else if tool_present("gnome-control-center") {
        spawn_shell("gnome-control-center screen");
    } else {
        log().warning("/screensaversettings: no systemsettings or gnome-control-center");
    }
}

// --- Input/clipboard backends (enigo / arboard) -------------------------------
// `pyautogui.press/hotkey` → enigo `key()` clicks; `keyboard.write` →
// enigo `text()`; `pyperclip.copy` → arboard. pyautogui key names map to
// enigo `Key` below (`Key::Unicode` covers single characters everywhere).

use enigo::{Direction, Enigo, Key, Keyboard, Settings};

fn enigo_agent() -> Result<Enigo, String> {
    Enigo::new(&Settings::default()).map_err(|e| e.to_string())
}

/// Map a pyautogui/`keyboard` key name to enigo. Returns `None` when the
/// name is better sent as typed text (single char) — handled by callers.
fn map_key(name: &str) -> Option<Key> {
    let lower = name.to_lowercase();
    // Letters/digits exist as Key variants on Windows only; elsewhere they
    // are typed as Unicode text (same visible effect as pyautogui.press).
    #[cfg(windows)]
    {
        if lower.len() == 1 {
            let ch = lower.chars().next().unwrap_or('\0');
            let key = match ch {
                'a' => Key::A,
                'b' => Key::B,
                'c' => Key::C,
                'd' => Key::D,
                'e' => Key::E,
                'f' => Key::F,
                'g' => Key::G,
                'h' => Key::H,
                'i' => Key::I,
                'j' => Key::J,
                'k' => Key::K,
                'l' => Key::L,
                'm' => Key::M,
                'n' => Key::N,
                'o' => Key::O,
                'p' => Key::P,
                'q' => Key::Q,
                'r' => Key::R,
                's' => Key::S,
                't' => Key::T,
                'u' => Key::U,
                'v' => Key::V,
                'w' => Key::W,
                'x' => Key::X,
                'y' => Key::Y,
                'z' => Key::Z,
                '0' => Key::Num0,
                '1' => Key::Num1,
                '2' => Key::Num2,
                '3' => Key::Num3,
                '4' => Key::Num4,
                '5' => Key::Num5,
                '6' => Key::Num6,
                '7' => Key::Num7,
                '8' => Key::Num8,
                '9' => Key::Num9,
                _ => return None,
            };
            return Some(key);
        }
    }
    match lower.as_str() {
        "ctrl" | "control" | "lctrl" => Some(Key::LControl),
        "rctrl" => Some(Key::RControl),
        "shift" | "lshift" => Some(Key::LShift),
        "rshift" => Some(Key::RShift),
        "alt" | "lalt" => Some(Key::Alt),
        "enter" | "return" => Some(Key::Return),
        "esc" | "escape" => Some(Key::Escape),
        "tab" => Some(Key::Tab),
        "space" => Some(Key::Space),
        "backspace" => Some(Key::Backspace),
        "delete" | "del" => Some(Key::Delete),
        "insert" | "ins" => Some(Key::Insert),
        "home" => Some(Key::Home),
        "end" => Some(Key::End),
        "pageup" | "pgup" => Some(Key::PageUp),
        "pagedown" | "pgdn" => Some(Key::PageDown),
        "up" => Some(Key::UpArrow),
        "down" => Some(Key::DownArrow),
        "left" => Some(Key::LeftArrow),
        "right" => Some(Key::RightArrow),
        "win" | "super" | "meta" | "lwin" | "windows" => Some(Key::Meta),
        "rwin" => Some(Key::Meta),
        "capslock" => Some(Key::CapsLock),
        "numlock" => Some(Key::Numlock),
        #[cfg(target_os = "windows")]
        "scrolllock" => Some(Key::Scroll),
        #[cfg(not(target_os = "windows"))]
        "scrolllock" => Some(Key::ScrollLock),
        "printscreen" | "prtsc" => Some(Key::PrintScr),
        "pause" => Some(Key::Pause),
        "volumemute" => Some(Key::VolumeMute),
        "volumeup" => Some(Key::VolumeUp),
        "volumedown" => Some(Key::VolumeDown),
        "playpause" | "medianext" | "mediaplaypause" => Some(Key::MediaPlayPause),
        "prevtrack" | "medianprevious" => Some(Key::MediaPrevTrack),
        "nexttrack" | "medianexttrack" => Some(Key::MediaNextTrack),
        "mediastop" => Some(Key::MediaStop),
        "f1" => Some(Key::F1),
        "f2" => Some(Key::F2),
        "f3" => Some(Key::F3),
        "f4" => Some(Key::F4),
        "f5" => Some(Key::F5),
        "f6" => Some(Key::F6),
        "f7" => Some(Key::F7),
        "f8" => Some(Key::F8),
        "f9" => Some(Key::F9),
        "f10" => Some(Key::F10),
        "f11" => Some(Key::F11),
        "f12" => Some(Key::F12),
        _ => None,
    }
}

fn press_key(key: &str) {
    match enigo_agent() {
        Ok(mut enigo) => {
            if let Some(mapped) = map_key(key) {
                if let Err(e) = enigo.key(mapped, Direction::Click) {
                    log().error(&format!("press_key({key:?}) failed: {e}"));
                }
            } else if key.chars().count() == 1 {
                // Single characters type directly (pyautogui.press parity).
                if let Err(e) = enigo.text(key) {
                    log().error(&format!("press_key({key:?}) failed: {e}"));
                }
            } else {
                log().warning(&format!("press_key({key:?}): unknown key name"));
            }
        }
        Err(e) => log().error(&format!("press_key backend unavailable: {e}")),
    }
}

fn hotkey(keys: &[&str]) {
    match enigo_agent() {
        Ok(mut enigo) => {
            let mut mapped: Vec<Key> = Vec::new();
            for key in keys {
                match map_key(key) {
                    Some(k) => mapped.push(k),
                    None if key.chars().count() == 1 => {
                        if let Some(ch) = key.chars().next() {
                            mapped.push(Key::Unicode(ch));
                        }
                    }
                    None => {
                        log().warning(&format!("hotkey({keys:?}): unknown key {key:?}"));
                        return;
                    }
                }
            }
            // pyautogui.hotkey: press all in order, release in reverse.
            for key in &mapped {
                if enigo.key(*key, Direction::Press).is_err() {
                    break;
                }
            }
            for key in mapped.iter().rev() {
                let _ = enigo.key(*key, Direction::Release);
            }
        }
        Err(e) => log().error(&format!("hotkey backend unavailable: {e}")),
    }
}

fn type_text(text: &str) {
    match enigo_agent() {
        Ok(mut enigo) => {
            if let Err(e) = enigo.text(text) {
                log().error(&format!("type_text failed: {e}"));
            }
        }
        Err(e) => log().error(&format!("type_text backend unavailable: {e}")),
    }
}

fn clipboard_copy(text: &str) {
    match arboard::Clipboard::new().and_then(|mut clipboard| clipboard.set_text(text.to_string())) {
        Ok(()) => log().debug("clipboard_copy: done"),
        Err(e) => log().error(&format!("clipboard_copy failed: {e}")),
    }
}

/// Port of the `/appvolume` branch — per-application volume via CoreAudio
/// session enumeration (`pycaw` equivalent). Argument parsing and volume
/// math mirror Python exactly (process-name match is case-insensitive).
/// Pure `/appvolume` target math (`set50`/`+5`/`-`/`-5`), shared by the
/// Windows (CoreAudio sessions) and Linux (`pactl` sink-inputs) branches.
/// `None` when the operator is unrecognized (Python would leave the volume
/// untouched via the same fall-through).
fn app_volume_target(command0: &str, old_percent: i32) -> Option<i32> {
    if command0.starts_with("set") {
        let mut target = command0.replace("set", "").parse::<i32>().ok()?;
        if target > 100 {
            target = 100;
        }
        if target < 0 {
            target = 0;
        }
        Some(target)
    } else if command0.starts_with('+') {
        let rest = command0.replace('+', "");
        if rest.is_empty() {
            Some(old_percent + 1)
        } else {
            Some(old_percent + rest.parse::<i32>().ok()?)
        }
    } else if command0.starts_with('-') {
        let rest = command0.replace('-', "");
        if rest.is_empty() {
            Some(old_percent - 1)
        } else {
            Some(old_percent - rest.parse::<i32>().ok()?)
        }
    } else {
        None
    }
}

/// Parse `pactl list sink-inputs` blocks into
/// `(index, process binary, volume percent)` triples.
#[cfg(target_os = "linux")]
fn parse_sink_inputs(output: &str) -> Vec<(u32, String, i32)> {
    let mut inputs = Vec::new();
    let mut index: Option<u32> = None;
    let mut binary: Option<String> = None;
    let mut percent: Option<i32> = None;
    let flush = |index: &mut Option<u32>,
                 binary: &mut Option<String>,
                 percent: &mut Option<i32>,
                 inputs: &mut Vec<(u32, String, i32)>| {
        if let (Some(index), Some(binary), Some(percent)) =
            (index.take(), binary.take(), percent.take())
        {
            inputs.push((index, binary, percent));
        }
    };
    for line in output.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("Sink Input #") {
            flush(&mut index, &mut binary, &mut percent, &mut inputs);
            index = rest.trim().parse::<u32>().ok();
        } else if let Some(value) = trimmed.strip_prefix("application.process.binary =") {
            binary = Some(value.trim().trim_matches('"').to_string());
        } else if trimmed.starts_with("Volume:") && percent.is_none() {
            percent = crate::app::buttons::audio::volume::parse_percent(trimmed);
        }
    }
    flush(&mut index, &mut binary, &mut percent, &mut inputs);
    inputs
}

fn app_volume(message: &str) {
    let normalized = message
        .replacen("/appvolume ", "", 1)
        .replace("set ", "set");
    let command: Vec<&str> = normalized.split_whitespace().collect();
    // Python IndexErrors here on missing args (→ HTTP 500); log instead.
    if command.len() < 2 {
        log().error("appvolume: missing process name");
        return;
    }

    #[cfg(windows)]
    {
        use windows::core::{Interface as _, PWSTR};
        use windows::Win32::Media::Audio::{
            eMultimedia, eRender, IAudioSessionControl2, IAudioSessionManager2,
            IMMDeviceEnumerator, ISimpleAudioVolume, MMDeviceEnumerator,
        };
        use windows::Win32::System::Com::{
            CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_ALL, COINIT_MULTITHREADED,
        };
        use windows::Win32::System::Threading::{
            OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
            PROCESS_QUERY_LIMITED_INFORMATION,
        };

        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        }
        let result = (|| -> windows::core::Result<()> {
            unsafe {
                let enumerator: IMMDeviceEnumerator =
                    CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
                let device = enumerator.GetDefaultAudioEndpoint(eRender, eMultimedia)?;
                let manager: IAudioSessionManager2 = device.Activate(CLSCTX_ALL, None)?;
                let sessions = manager.GetSessionEnumerator()?;
                let count = sessions.GetCount()?;
                for i in 0..count {
                    let Ok(control) = sessions.GetSession(i as i32) else {
                        continue;
                    };
                    let Ok(control2) = control.cast::<IAudioSessionControl2>() else {
                        continue;
                    };
                    let Ok(pid) = control2.GetProcessId() else {
                        continue;
                    };
                    if pid == 0 {
                        continue;
                    }
                    let Ok(process) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid)
                    else {
                        continue;
                    };
                    let process = ProcessGuard(process);
                    let mut buffer = [0u16; 512];
                    let mut length = buffer.len() as u32;
                    if QueryFullProcessImageNameW(
                        process.0,
                        PROCESS_NAME_WIN32,
                        PWSTR(buffer.as_mut_ptr()),
                        &mut length,
                    )
                    .is_err()
                    {
                        continue;
                    }
                    let image = String::from_utf16_lossy(&buffer[..length as usize]);
                    let process_name = image.rsplit(['/', '\\']).next().unwrap_or("");
                    if !process_name.eq_ignore_ascii_case(command[1]) {
                        continue;
                    }
                    let Ok(volume) = control.cast::<ISimpleAudioVolume>() else {
                        continue;
                    };
                    let old_volume = volume.GetMasterVolume().unwrap_or(0.0);
                    log().debug(&format!("Current volume: {old_volume}"));
                    let old_percent = (old_volume * 100.0).round() as i32;

                    let Some(target_volume) = app_volume_target(command[0], old_percent) else {
                        continue;
                    };

                    volume.SetMasterVolume(target_volume as f32 / 100.0, std::ptr::null())?;
                    log().debug(&format!(
                        "New volume: {}",
                        volume.GetMasterVolume().unwrap_or(-1.0)
                    ));
                }
                Ok(())
            }
        })();
        unsafe {
            CoUninitialize();
        }
        if let Err(e) = result {
            log().exception(&e, Some("appvolume failed"), true, true, true);
        }
    }
    // Per-process sink-input volumes via pactl (process-name match is
    // case-insensitive, like Python's CoreAudio session match).
    #[cfg(target_os = "linux")]
    {
        use crate::app::buttons::audio::volume::{pactl_output, pactl_run};
        let wanted = command[1].to_lowercase();
        match pactl_output(&["list", "sink-inputs"]) {
            Ok(out) => {
                for (index, binary, old_percent) in parse_sink_inputs(&out) {
                    if binary.to_lowercase() != wanted {
                        continue;
                    }
                    log().debug(&format!("Current volume: {}", old_percent as f32 / 100.0));
                    let Some(target) = app_volume_target(command[0], old_percent) else {
                        continue;
                    };
                    if pactl_run(&[
                        "set-sink-input-volume",
                        &index.to_string(),
                        &format!("{target}%"),
                    ])
                    .is_err()
                    {
                        log().warning(&format!("appvolume: failed to set {binary} to {target}%"));
                    } else {
                        log().debug(&format!("New volume: {}", target as f32 / 100.0));
                    }
                }
            }
            Err(e) => log().warning(&format!("appvolume: pactl unavailable: {e}")),
        }
    }
    #[cfg(not(any(windows, target_os = "linux")))]
    {
        log().warning(&format!(
            "appvolume {command:?}: per-app volume is only supported on Windows and Linux"
        ));
    }
}

/// RAII `CloseHandle` for process handles opened during session matching.
#[cfg(windows)]
struct ProcessGuard(windows::Win32::Foundation::HANDLE);

#[cfg(windows)]
impl Drop for ProcessGuard {
    fn drop(&mut self) {
        unsafe {
            let _ = windows::Win32::Foundation::CloseHandle(self.0);
        }
    }
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
        #[cfg(windows)]
        spawn_shell("shutdown /s /f /t 0");
        #[cfg(target_os = "linux")]
        spawn_shell("systemctl poweroff");
        #[cfg(not(any(windows, target_os = "linux")))]
        log().warning("/PCshutdown: only supported on Windows and Linux");
    } else if message.starts_with("/PCrestart") {
        #[cfg(windows)]
        spawn_shell("shutdown /r /f /t 0");
        #[cfg(target_os = "linux")]
        spawn_shell("systemctl reboot");
        #[cfg(not(any(windows, target_os = "linux")))]
        log().warning("/PCrestart: only supported on Windows and Linux");
    } else if message.starts_with("/PCsleep") {
        #[cfg(windows)]
        spawn_shell("rundll32.exe powrprof.dll,SetSuspendState 0,1,0");
        #[cfg(target_os = "linux")]
        spawn_shell("systemctl suspend");
        #[cfg(not(any(windows, target_os = "linux")))]
        log().warning("/PCsleep: only supported on Windows and Linux");
    } else if message.starts_with("/PChibernate") {
        #[cfg(windows)]
        spawn_shell("shutdown /h /t 0");
        #[cfg(target_os = "linux")]
        spawn_shell("systemctl hibernate");
        #[cfg(not(any(windows, target_os = "linux")))]
        log().warning("/PChibernate: only supported on Windows and Linux");
    } else if message.starts_with("/locksession") {
        #[cfg(windows)]
        spawn_shell("Rundll32.exe user32.dll,LockWorkStation");
        // logind locks every compositor; fall back to the ScreenSaver bus API.
        #[cfg(target_os = "linux")]
        if !(tool_present("loginctl") && run_shell_checked("loginctl lock-session")) {
            screensaver_dbus("Lock", "");
        }
        #[cfg(not(any(windows, target_os = "linux")))]
        log().warning("/locksession: only supported on Windows and Linux");
    } else if message.starts_with("/screensaversettings") {
        #[cfg(windows)]
        spawn_shell("rundll32.exe desk.cpl,InstallScreenSaver toasters.scr");
        #[cfg(target_os = "linux")]
        linux_screensaver_settings();
        #[cfg(not(any(windows, target_os = "linux")))]
        log().warning("/screensaversettings: only supported on Windows and Linux");
    } else if message.starts_with("/screensaver") && !message.starts_with("/screensaversettings") {
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
    } else if message.starts_with("/key") {
        let key = message.replacen("/key", "", 1);
        press_key(key.trim());
    } else if message.starts_with("/restartexplorer") {
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
    } else if message.starts_with("/kill")
        || message.starts_with("/taskill")
        || message.starts_with("/taskkill")
        || message.starts_with("/forceclose")
    {
        let window_name = message
            .replace("/kill", "")
            .replace("/taskill", "")
            .replace("/taskkill", "")
            .replace("/forceclose", "");
        match window::get_by_name(&window_name) {
            Ok(hwnd) => log().debug(&format!("Window '{window_name}' found with handle: {hwnd}")),
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
    } else if message.starts_with("/restart") {
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
    } else if message.starts_with("/clearclipboard") {
        #[cfg(windows)]
        spawn_shell("cmd /c \"echo off | clip\"");
        // arboard clears the clipboard on every platform (wl-copy/xclip
        // backed on Linux); more reliable than shelling out.
        #[cfg(not(windows))]
        clipboard_copy("");
    } else if message.starts_with("/write ") {
        type_text(&message.replacen("/write ", "", 1));
    } else if message.starts_with("/writeandsend ") {
        type_text(&message.replacen("/writeandsend ", "", 1));
        press_key("ENTER");
    } else if message.starts_with("/appvolume +")
        || message.starts_with("/appvolume -")
        || message.starts_with("/appvolume set")
    {
        app_volume(&message);
    } else if message.starts_with("/soundcontrol mute") {
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
    } else if message.starts_with("/mediacontrol playpause") {
        #[cfg(windows)]
        press_key("playpause");
        #[cfg(target_os = "linux")]
        mpris_command("PlayPause");
        #[cfg(not(any(windows, target_os = "linux")))]
        press_key("playpause");
    } else if message.starts_with("/mediacontrol previous") {
        #[cfg(windows)]
        press_key("prevtrack");
        #[cfg(target_os = "linux")]
        mpris_command("Previous");
        #[cfg(not(any(windows, target_os = "linux")))]
        press_key("prevtrack");
    } else if message.starts_with("/mediacontrol next") {
        #[cfg(windows)]
        press_key("nexttrack");
        #[cfg(target_os = "linux")]
        mpris_command("Next");
        #[cfg(not(any(windows, target_os = "linux")))]
        press_key("nexttrack");
    } else if message.starts_with("/speechrecognition") {
        hotkey(&["win", "h"]);
    } else if message.starts_with("/superAltF4") {
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
    } else if message.starts_with("/firstplan") {
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
        if let Err(message) = exec::python(&message) {
            return failure(&message);
        }
    } else if message.starts_with("/batch") {
        if let Err(message) = exec::batch(&message) {
            return failure(&message);
        }
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

    success()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_common_key_names() {
        assert!(matches!(map_key("ctrl"), Some(Key::LControl)));
        assert!(matches!(map_key("ENTER"), Some(Key::Return)));
        assert!(matches!(map_key("volumemute"), Some(Key::VolumeMute)));
        assert!(matches!(map_key("playpause"), Some(Key::MediaPlayPause)));
        assert!(matches!(map_key("win"), Some(Key::Meta)));
        assert!(matches!(map_key("f12"), Some(Key::F12)));
        assert_eq!(map_key("not-a-key-zzz"), None);
        // Single chars map on Windows (Key variants), type as text elsewhere.
        #[cfg(windows)]
        assert!(map_key("h").is_some());
        #[cfg(not(windows))]
        assert_eq!(map_key("h"), None);
    }

    #[test]
    fn app_volume_math_matches_python() {
        assert_eq!(app_volume_target("set50", 20), Some(50));
        assert_eq!(app_volume_target("set150", 20), Some(100));
        assert_eq!(app_volume_target("set-5", 20), Some(0));
        assert_eq!(app_volume_target("+5", 50), Some(55));
        assert_eq!(app_volume_target("+", 50), Some(51));
        assert_eq!(app_volume_target("-5", 50), Some(45));
        assert_eq!(app_volume_target("-", 50), Some(49));
        // No clamp on +/- (Python only clamps set).
        assert_eq!(app_volume_target("+100", 50), Some(150));
        // Garbage → skip.
        assert_eq!(app_volume_target("set", 20), None);
        assert_eq!(app_volume_target("+x", 20), None);
        assert_eq!(app_volume_target("mute", 20), None);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn sink_inputs_parse_index_binary_percent() {
        let sample = "Sink Input #19638\n\tMute: no\n\tVolume: front-left: 51118 /  78% / -6,47 dB\n\tProperties:\n\t\tapplication.process.binary = \"firefox\"\nSink Input #7\n\tVolume: mono: 1 /  0%\n\t\tapplication.process.binary = \"x\"\n";
        assert_eq!(
            parse_sink_inputs(sample),
            vec![(19638, "firefox".to_string(), 78), (7, "x".to_string(), 0),]
        );
        assert!(parse_sink_inputs("").is_empty());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn shell_quote_wraps_safely() {
        assert_eq!(shell_quote("firefox"), "'firefox'");
        assert_eq!(shell_quote("a'b"), "'a'\\''b'");
    }
}
