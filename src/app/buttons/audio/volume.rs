//! Port of `app/buttons/audio/volume.py`.
//!
//! `pycaw` maps to `windows`-crate CoreAudio (`MMDeviceEnumerator` →
//! `IAudioEndpointVolume`); `win32api.keybd_event` maps to `keybd_event`.
//! The `set_volume` stepping loop and `increase`/`decrease` delta math are
//! ported exactly, with two hang-hardening deviations: the target is clamped
//! to `[0.0, 1.0]` and the stepping loop is capped (upstream loops forever
//! on out-of-range targets).
//!
//! On Linux the same shapes run against `pactl` (PulseAudio/PipeWire);
//! other platforms return errors.

use crate::app::utils::logger::log;

#[cfg(windows)]
mod imp {
    use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
    use windows::Win32::Media::Audio::{
        eConsole, eRender, IMMDeviceEnumerator, MMDeviceEnumerator,
    };
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_ALL, COINIT_MULTITHREADED,
    };
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        keybd_event, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP, VK_VOLUME_DOWN, VK_VOLUME_UP,
    };

    fn endpoint_volume() -> windows::core::Result<IAudioEndpointVolume> {
        unsafe {
            let enumerator: IMMDeviceEnumerator =
                CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
            let device = enumerator.GetDefaultAudioEndpoint(eRender, eConsole)?;
            device.Activate(CLSCTX_ALL, None)
        }
    }

    pub fn get_current_volume() -> Result<f32, String> {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        }
        let result = (|| -> windows::core::Result<f32> {
            unsafe {
                let volume = endpoint_volume()?;
                Ok(volume.GetMasterVolumeLevelScalar()?)
            }
        })();
        unsafe {
            CoUninitialize();
        }
        result.map_err(|e| e.to_string())
    }

    fn set_scalar(value: f32) -> windows::core::Result<()> {
        unsafe {
            endpoint_volume()?.SetMasterVolumeLevelScalar(value, std::ptr::null())?;
        }
        Ok(())
    }

    fn is_close(a: f32, b: f32) -> bool {
        // math.isclose(a, b, rel_tol=0.01)
        (a - b).abs() <= 0.01 * a.abs().max(b.abs())
    }

    pub fn set_volume(target_volume: f32) -> Result<f32, String> {
        let target = target_volume.clamp(0.0, 1.0);
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        }
        let mut current = get_current_volume()?;
        for _ in 0..250 {
            if is_close(current, target) {
                break;
            }
            if current > target {
                current -= 0.01;
            } else {
                current += 0.01;
            }
            if let Err(e) = set_scalar(current) {
                unsafe {
                    CoUninitialize();
                }
                return Err(e.to_string());
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        unsafe {
            CoUninitialize();
        }
        Ok(current)
    }

    pub fn volume_key(up: bool) {
        unsafe {
            let key: u8 = if up {
                VK_VOLUME_UP.0 as u8
            } else {
                VK_VOLUME_DOWN.0 as u8
            };
            keybd_event(key, 0, KEYBD_EVENT_FLAGS(0), 0);
            keybd_event(key, 0, KEYEVENTF_KEYUP, 0);
        }
    }
}

/// Linux backend: default-sink volume via `pactl` (works on PulseAudio
/// and on PipeWire through pipewire-pulse).
#[cfg(target_os = "linux")]
mod imp_linux {
    use super::{pactl_output, pactl_run, parse_percent};

    pub fn get_current_volume() -> Result<f32, String> {
        let out = pactl_output(&["get-sink-volume", "@DEFAULT_SINK@"])?;
        parse_percent(&out)
            .map(|p| p as f32 / 100.0)
            .ok_or_else(|| "could not parse pactl volume".to_string())
    }

    pub fn set_volume(target_volume: f32) -> Result<f32, String> {
        // Same stepping shape as Windows (1% steps, isclose gate, capped):
        // pactl rounds to integer percent, so stepping lands exactly.
        let target = (target_volume.clamp(0.0, 1.0) * 100.0).round() as i32;
        let mut current = get_current_volume()?;
        for _ in 0..250 {
            if is_close(current, target as f32 / 100.0) {
                break;
            }
            current = if current > target as f32 / 100.0 {
                current - 0.01
            } else {
                current + 0.01
            };
            let percent = (current.clamp(0.0, 1.0) * 100.0).round() as i32;
            pactl_run(&["set-sink-volume", "@DEFAULT_SINK@", &format!("{percent}%")])?;
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        Ok(current)
    }

    /// Port of the `VK_VOLUME_UP`/`VK_VOLUME_DOWN` key press: a 2% nudge
    /// (the Windows system step) before the absolute math runs.
    pub fn volume_key(up: bool) {
        let step = if up { "+2%" } else { "-2%" };
        if pactl_run(&["set-sink-volume", "@DEFAULT_SINK@", step]).is_err() {
            crate::app::utils::logger::log().debug("volume_key: pactl not available");
        }
    }

    fn is_close(a: f32, b: f32) -> bool {
        (a - b).abs() <= 0.01 * a.abs().max(b.abs())
    }
}

/// Run `pactl`, returning stdout on success.
#[cfg(target_os = "linux")]
pub(crate) fn pactl_output(args: &[&str]) -> Result<String, String> {
    let output = std::process::Command::new("pactl")
        .args(args)
        .output()
        .map_err(|e| format!("pactl unavailable: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "pactl {args:?} failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Run `pactl`, checking only the exit status.
#[cfg(target_os = "linux")]
pub(crate) fn pactl_run(args: &[&str]) -> Result<(), String> {
    pactl_output(args).map(|_| ())
}

/// Parse the first `NN%` in `pactl` volume output
/// (`Volume: front-left: 51773 /  79% / …`).
#[cfg(target_os = "linux")]
pub(crate) fn parse_percent(output: &str) -> Option<i32> {
    let idx = output.find('%')?;
    output[..idx]
        .split(|c: char| !c.is_ascii_digit())
        .filter(|s| !s.is_empty())
        .next_back()?
        .parse::<i32>()
        .ok()
}

/// Parse `pactl list sinks|sources` blocks into `(name, description)`
/// pairs. `header` is `"Sink #"` or `"Source #"`; the description is
/// pactl's `FriendlyName` equivalent (matched exactly, like Python).
#[cfg(target_os = "linux")]
pub(crate) fn parse_endpoint_names(output: &str, header: &str) -> Vec<(String, String)> {
    let mut endpoints = Vec::new();
    let mut name: Option<String> = None;
    let mut description: Option<String> = None;
    let mut in_block = false;
    for line in output.lines() {
        if line.starts_with(header) {
            if let (Some(name), Some(description)) = (name.take(), description.take()) {
                endpoints.push((name, description));
            }
            in_block = true;
        } else if in_block {
            let trimmed = line.trim();
            if let Some(value) = trimmed.strip_prefix("Name:") {
                name = Some(value.trim().to_string());
            } else if let Some(value) = trimmed.strip_prefix("Description:") {
                description = Some(value.trim().to_string());
            }
        }
    }
    if let (Some(name), Some(description)) = (name, description) {
        endpoints.push((name, description));
    }
    endpoints
}

/// Port of `get_current_volume`.
pub fn get_current_volume() -> Result<f32, String> {
    #[cfg(windows)]
    {
        imp::get_current_volume()
    }
    #[cfg(target_os = "linux")]
    {
        imp_linux::get_current_volume()
    }
    #[cfg(not(any(windows, target_os = "linux")))]
    {
        Err("get_current_volume is only supported on Windows and Linux".to_string())
    }
}

/// Port of `set_volume`.
pub fn set_volume(target_volume: f32) -> Result<f32, String> {
    #[cfg(windows)]
    {
        imp::set_volume(target_volume)
    }
    #[cfg(target_os = "linux")]
    {
        imp_linux::set_volume(target_volume)
    }
    #[cfg(not(any(windows, target_os = "linux")))]
    {
        let _ = target_volume;
        Err("set_volume is only supported on Windows and Linux".to_string())
    }
}

/// Port of `increase_volume` — key event first (always, like Python), then
/// the `int(delta)` math.
pub fn increase_volume(delta: &str) -> Result<f32, String> {
    #[cfg(windows)]
    imp::volume_key(true);
    #[cfg(target_os = "linux")]
    imp_linux::volume_key(true);
    #[cfg(not(any(windows, target_os = "linux")))]
    log().debug("increase_volume: volume keys are only supported on Windows and Linux");
    if delta.trim().is_empty() {
        return get_current_volume();
    }
    let current = get_current_volume()?;
    let delta = delta
        .trim()
        .parse::<i32>()
        .map_err(|e| format!("Invalid volume delta {delta:?}: {e}"))?;
    set_volume(current + delta as f32 / 100.0)
}

/// Port of `decrease_volume` — key event first (always, like Python), then
/// the `int(delta)` math.
pub fn decrease_volume(delta: &str) -> Result<f32, String> {
    #[cfg(windows)]
    imp::volume_key(false);
    #[cfg(target_os = "linux")]
    imp_linux::volume_key(false);
    #[cfg(not(any(windows, target_os = "linux")))]
    log().debug("decrease_volume: volume keys are only supported on Windows and Linux");
    if delta.trim().is_empty() {
        return get_current_volume();
    }
    let current = get_current_volume()?;
    let delta = delta
        .trim()
        .parse::<i32>()
        .map_err(|e| format!("Invalid volume delta {delta:?}: {e}"))?;
    set_volume(current - delta as f32 / 100.0)
}

/// Port of `handle_command` (re-exported as `audio::change_volume`).
pub fn handle_command(message: &str) {
    if message.starts_with("/volume +") {
        let delta = message.replacen("/volume +", "", 1);
        if delta.replace(' ', "").is_empty() {
            let _ = increase_volume("1");
        } else {
            let _ = increase_volume(&delta);
        }
    } else if message.starts_with("/volume -") {
        let delta = message.replacen("/volume -", "", 1);
        if delta.replace(' ', "").is_empty() {
            let _ = decrease_volume("1");
        } else {
            let _ = decrease_volume(&delta);
        }
    } else if message.starts_with("/volume set") {
        match message
            .replacen("/volume set ", "", 1)
            .trim()
            .parse::<i32>()
        {
            Ok(target) => {
                let _ = set_volume(target as f32 / 100.0);
            }
            Err(e) => log().error(&format!("Invalid /volume set target: {e}")),
        }
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;

    #[test]
    fn percent_parses_first_channel() {
        assert_eq!(
            parse_percent("Volume: front-left: 51773 /  79% / -6,14 dB,   front-right: 51773 /  79% / -6,14 dB"),
            Some(79)
        );
        assert_eq!(parse_percent("Volume: mono: 65536 / 100%"), Some(100));
        assert_eq!(parse_percent("Volume: muted"), None);
        assert_eq!(parse_percent(""), None);
    }

    #[test]
    fn endpoints_parse_name_and_description() {
        let sample = "Sink #13631\n\tState: RUNNING\n\tName: alsa_output.usb-UGREEN.analog-stereo\n\tDescription: UGREEN Studio Pro\nSink #7\n\tName: combined\n\tDescription: Simultaneous output\n";
        assert_eq!(
            parse_endpoint_names(sample, "Sink #"),
            vec![
                (
                    "alsa_output.usb-UGREEN.analog-stereo".to_string(),
                    "UGREEN Studio Pro".to_string()
                ),
                ("combined".to_string(), "Simultaneous output".to_string()),
            ]
        );
        // Wrong header matches nothing; incomplete blocks are skipped.
        assert!(parse_endpoint_names(sample, "Source #").is_empty());
        assert!(parse_endpoint_names("Source #1\n\tName: x\n", "Source #").is_empty());
    }
}
