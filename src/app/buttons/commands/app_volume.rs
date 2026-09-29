//! Per-application volume command (extracted from `commands.rs`).
//!
//! Port of the `/appvolume` branch — per-application volume via CoreAudio
//! session enumeration (`pycaw` equivalent) on Windows and `pactl`
//! sink-inputs on Linux. Argument parsing and volume math mirror Python
//! exactly (process-name match is case-insensitive).

use crate::app::utils::logger::log;

/// Pure `/appvolume` target math (`set50`/`+5`/`-`/`-5`), shared by the
/// Windows (CoreAudio sessions) and Linux (`pactl` sink-inputs) branches.
/// `None` when the operator is unrecognized (Python would leave the volume
/// untouched via the same fall-through).
fn app_volume_target(command0: &str, old_percent: i32) -> Option<i32> {
    if command0.starts_with("set") {
        let target = command0.replace("set", "").parse::<i32>().ok()?;
        Some(target.clamp(0, 100))
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

pub(crate) fn app_volume(message: &str) {
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
