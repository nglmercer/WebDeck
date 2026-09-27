//! Port of `app/buttons/audio/volume.py`.
//!
//! `pycaw` maps to `windows`-crate CoreAudio (`MMDeviceEnumerator` →
//! `IAudioEndpointVolume`); `win32api.keybd_event` maps to `keybd_event`.
//! The `set_volume` stepping loop and `increase`/`decrease` delta math are
//! ported exactly, with two hang-hardening deviations: the target is clamped
//! to `[0.0, 1.0]` and the stepping loop is capped (upstream loops forever
//! on out-of-range targets).
//!
//! Non-Windows platforms return errors (endpoint volume is OS-specific).

use crate::app::utils::logger::log;

#[cfg(windows)]
mod imp {
    use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
    use windows::Win32::Media::Audio::{IMMDeviceEnumerator, MMDeviceEnumerator, eConsole, eRender};
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
            let key: u8 = if up { VK_VOLUME_UP.0 as u8 } else { VK_VOLUME_DOWN.0 as u8 };
            keybd_event(key, 0, KEYBD_EVENT_FLAGS(0), 0);
            keybd_event(key, 0, KEYEVENTF_KEYUP, 0);
        }
    }
}

/// Port of `get_current_volume`.
pub fn get_current_volume() -> Result<f32, String> {
    #[cfg(windows)]
    {
        imp::get_current_volume()
    }
    #[cfg(not(windows))]
    {
        Err("get_current_volume is only supported on Windows".to_string())
    }
}

/// Port of `set_volume`.
pub fn set_volume(target_volume: f32) -> Result<f32, String> {
    #[cfg(windows)]
    {
        imp::set_volume(target_volume)
    }
    #[cfg(not(windows))]
    {
        let _ = target_volume;
        Err("set_volume is only supported on Windows".to_string())
    }
}

/// Port of `increase_volume` — key event first (always, like Python), then
/// the `int(delta)` math.
pub fn increase_volume(delta: &str) -> Result<f32, String> {
    #[cfg(windows)]
    imp::volume_key(true);
    #[cfg(not(windows))]
    log().debug("increase_volume: volume keys are only supported on Windows");
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
    #[cfg(not(windows))]
    log().debug("decrease_volume: volume keys are only supported on Windows");
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
        match message.replacen("/volume set ", "", 1).trim().parse::<i32>() {
            Ok(target) => {
                let _ = set_volume(target as f32 / 100.0);
            }
            Err(e) => log().error(&format!("Invalid /volume set target: {e}")),
        }
    }
}
