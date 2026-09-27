//! Port of `app/buttons/soundboard/devices.py`.
//!
//! Python enumerates output devices through libVLC
//! (`audio_output_device_enum`) and returns the VLC device id whose
//! description contains the configured name. The Rust backend is `rodio`,
//! so this returns the matching `cpal` output-device *name* instead — the
//! handle [`super::player`] needs to open that same device. Matching rule
//! (case-insensitive substring) is unchanged.
//!
//! Where Python returns `None` (no match) or raises `RuntimeError`
//! (enumeration failed), this returns `Err`; callers treat `Err` as falsy,
//! exactly like the Python `if cable_input_device:` gates.

use rodio::cpal::traits::{DeviceTrait, HostTrait};

use crate::app::utils::languages::text;
use crate::app::utils::logger::log;

/// Port of `get_device`: configured-name → live output-device name.
pub fn get_device(device: &str) -> Result<String, String> {
    let host = rodio::cpal::default_host();
    let devices = match host.output_devices() {
        Ok(devices) => devices,
        Err(e) => {
            log().exception(
                &e,
                Some(&format!(
                    "Failed to retrieve audio device '{device}'. Ensure VLC is installed properly."
                )), true, true, true,
            );
            return Err(text(Some("vlc_not_installed_error"), None));
        }
    };
    for candidate in devices {
        let name = candidate
            .description()
            .map(|d| d.name().to_string())
            .unwrap_or_default();
        // Python: `device.lower() in str(mod.description).lower()`.
        if name.to_lowercase().contains(&device.to_lowercase()) {
            return Ok(name);
        }
    }
    Err(format!("Failed to retrieve audio device '{device}'."))
}
