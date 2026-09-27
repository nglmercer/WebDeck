//! Port of `app/utils/settings/audio_devices.py`.
//!
//! `pyaudio` maps to `cpal`. The `maxInputChannels`/`maxOutputChannels`
//! selection maps to `input_devices()`/`output_devices()`; the
//! `hostApi == 0` check has no `cpal` equivalent and is intentionally
//! dropped (host selection is implicit). Name de-dup and the
//! "microsoft - input" skip are kept 1:1.

use cpal::traits::HostTrait;

use crate::app::utils::logger::log;

/// Port of `get_audio_devices`.
pub fn get_audio_devices(channels_type: &str) -> Vec<String> {
    let mut all_devices: Vec<String> = Vec::new();

    let result = (|| -> Result<(), String> {
        let host = cpal::default_host();
        if channels_type == "input" {
            let devices = host.input_devices().map_err(|e| e.to_string())?;
            for device in devices {
                let name = device.to_string();
                consider_device(&mut all_devices, &name);
            }
        } else {
            let devices = host.output_devices().map_err(|e| e.to_string())?;
            for device in devices {
                let name = device.to_string();
                consider_device(&mut all_devices, &name);
            }
        }
        Ok(())
    })();

    if let Err(e) = result {
        log().exception(
            &e,
            Some("An error has occurred while retrieving audio devices"),
            true,
            true,
            true,
        );
    }
    all_devices
}

fn after_paren(s: &str) -> &str {
    match s.find('(') {
        Some(i) => &s[i + 1..],
        None => s,
    }
}

/// Port of the inner per-device filter (substring de-dup + microsoft skip).
fn consider_device(all_devices: &mut Vec<String>, name: &str) {
    let mut ok = true;
    for device in all_devices.iter() {
        if after_paren(name).contains(after_paren(device)) {
            ok = false;
        }
    }
    // NOTE: Python checks `not "microsoft - input" in name` for both input
    // and output lists; kept 1:1.
    if ok && !name.to_lowercase().contains("microsoft - input") {
        all_devices.push(name.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enumerates_without_panicking() {
        // Hardware-dependent: only assert it runs and returns sane values.
        let _ = get_audio_devices("input");
        let _ = get_audio_devices("output");
    }

    #[test]
    fn dedup_and_microsoft_filter() {
        // Python skips when the EXISTING suffix is contained in the NEW one.
        let mut devices = Vec::new();
        consider_device(&mut devices, "Microphone (USB)");
        consider_device(&mut devices, "Headset (Big USB) Plus");
        assert_eq!(devices.len(), 1);
        consider_device(&mut devices, "Microsoft - Input Mapper");
        assert_eq!(devices.len(), 1);
        consider_device(&mut devices, "Speakers (Realtek)");
        assert_eq!(devices.len(), 2);
    }
}
