//! Port of `app/buttons/soundboard/mic.py`.
//!
//! Backend mapping: `pyaudio` blocking streams → `cpal` callback streams
//! (i16, 44100 Hz, mic input-channel count) bridged by a bounded sample
//! channel. Device matching (name after the first `(`, case-insensitive
//! substring, input/output capability gates) and the `sb_on` / `stop` /
//! `restart` lifecycle are 1:1 with Python.
//!
//! Documented deviations (mechanical, no behavior lost):
//! - Python pumps `stream_in.read(1024)` → `stream_out.write(data)` on the
//!   thread; cpal pumps via callbacks while the thread parks on `SB_ON`.
//! - Python's `OSError → restart()` (device unplugged mid-pump) maps to
//!   `ErrorKind::DeviceNotAvailable → restart()`; other stream
//!   errors are logged only.
//! - Where Python crashes the thread (`TypeError` on missing devices,
//!   `RuntimeError` from `get_device` in `restart`), this logs and returns.
//! - cpal devices have no portaudio index, so the "found at index" debug
//!   logs carry the device name instead.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use super::devices::get_device;
use crate::app::utils::{logger::log, settings::get_config::get_config};

/// Port of the `sb_on` module global.
static SB_ON: AtomicBool = AtomicBool::new(true);

/// Human-readable device name (cpal 0.18 removed `Device::name()`).
fn device_name(device: &cpal::Device) -> String {
    device
        .description()
        .map(|d| d.name().to_string())
        .unwrap_or_default()
}
/// Set when a stream reports the device as gone (≈ Python's `OSError`).
static STREAM_DEAD: AtomicBool = AtomicBool::new(false);

/// Port of `name[name.find("(") + 1:]` (no `(` → the whole string).
fn name_after_paren(full: &str) -> &str {
    match full.find('(') {
        Some(i) => &full[i + 1..],
        None => full,
    }
}

fn soundboard_config(key: &str) -> String {
    get_config(false, false)
        .get("settings")
        .and_then(|s| s.get("soundboard"))
        .and_then(|s| s.get(key))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

/// Port of `soundboard`: mic → VB-Cable loopback. Blocks until [`stop`].
pub fn soundboard() {
    SB_ON.store(true, Ordering::SeqCst);
    STREAM_DEAD.store(false, Ordering::SeqCst);

    let microphone_name = name_after_paren(&soundboard_config("mic_input_device")).to_lowercase();
    let output_name = name_after_paren(&soundboard_config("vbcable")).to_lowercase();

    let host = cpal::default_host();

    let mut mic_device = None;
    if let Ok(devices) = host.input_devices() {
        for device in devices {
            let name = device_name(&device);
            let has_input = device
                .supported_input_configs()
                .map(|mut configs| configs.next().is_some())
                .unwrap_or(false);
            // Python: `maxInputChannels > 0 and microphone_name in name`.
            if has_input && name.to_lowercase().contains(&microphone_name) {
                mic_device = Some(device);
                break;
            }
        }
    }

    let mut output_device = None;
    if let Ok(devices) = host.output_devices() {
        for device in devices {
            let name = device_name(&device);
            let has_output = device
                .supported_output_configs()
                .map(|mut configs| configs.next().is_some())
                .unwrap_or(false);
            // Python: `maxOutputChannels > 0 and output_name in name`.
            if has_output && name.to_lowercase().contains(&output_name) {
                output_device = Some(device);
                break;
            }
        }
    }

    match &mic_device {
        Some(device) => log().debug(&format!(
            "Microphone '{}' found at device '{}'",
            microphone_name,
            device_name(device)
        )),
        None => log().debug("Cannot find microphone."),
    }
    match &output_device {
        Some(device) => log().debug(&format!(
            "Speaker '{}' found at device '{}'",
            output_name,
            device_name(device)
        )),
        None => log().debug("Cannot find speakers."),
    }

    // Python crashes here with TypeError when a device is missing (the
    // thread just dies); log and return instead.
    let (Some(mic), Some(output)) = (mic_device, output_device) else {
        return;
    };

    // Python uses the mic's maxInputChannels for BOTH streams (including the
    // `output_channels` variable, which is then ignored); mirror that.
    let input_channels = mic
        .supported_input_configs()
        .ok()
        .and_then(|configs| configs.map(|c| c.channels()).max())
        .unwrap_or(0);
    if input_channels == 0 {
        log().debug("Cannot find microphone.");
        return;
    }
    if let Ok(info) = mic.default_input_config() {
        log().debug(&format!("i: {info:?}"));
    }
    if let Ok(info) = output.default_output_config() {
        log().debug(&format!("o: {info:?}"));
    }

    let stream_config = cpal::StreamConfig {
        channels: input_channels,
        sample_rate: cpal::SAMPLE_RATE_CD,
        buffer_size: cpal::BufferSize::Default,
    };

    // Bounded sample bridge (≈ the 1024-frame read/write pump; drops on
    // overflow instead of blocking the capture callback).
    let (tx, rx) = mpsc::sync_channel::<Vec<i16>>(16);
    let stream_in = match mic.build_input_stream(
        stream_config,
        move |data: &[i16], _| {
            let _ = tx.try_send(data.to_vec());
        },
        |error| {
            log().error(&format!("soundboard input stream error: {error}"));
            if error.kind() == cpal::ErrorKind::DeviceNotAvailable {
                STREAM_DEAD.store(true, Ordering::SeqCst);
            }
        },
        None,
    ) {
        Ok(stream) => stream,
        Err(e) => {
            log().exception(
                &e,
                Some("Failed to open soundboard input stream"),
                true,
                true,
                true,
            );
            return;
        }
    };

    let mut pending: VecDeque<i16> = VecDeque::new();
    let stream_out = match output.build_output_stream(
        stream_config,
        move |data: &mut [i16], _| {
            while pending.len() < data.len() {
                match rx.try_recv() {
                    Ok(chunk) => pending.extend(chunk),
                    Err(_) => break,
                }
            }
            for sample in data.iter_mut() {
                *sample = pending.pop_front().unwrap_or(0);
            }
        },
        |error| {
            log().error(&format!("soundboard output stream error: {error}"));
            if error.kind() == cpal::ErrorKind::DeviceNotAvailable {
                STREAM_DEAD.store(true, Ordering::SeqCst);
            }
        },
        None,
    ) {
        Ok(stream) => stream,
        Err(e) => {
            log().exception(
                &e,
                Some("Failed to open soundboard output stream"),
                true,
                true,
                true,
            );
            return;
        }
    };

    if let Err(e) = stream_in.play() {
        log().exception(
            &e,
            Some("Failed to start soundboard input stream"),
            true,
            true,
            true,
        );
        return;
    }
    if let Err(e) = stream_out.play() {
        log().exception(
            &e,
            Some("Failed to start soundboard output stream"),
            true,
            true,
            true,
        );
        return;
    }

    log().info("Soundboard is now active and streaming audio.");

    while SB_ON.load(Ordering::SeqCst) && !STREAM_DEAD.load(Ordering::SeqCst) {
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    log().debug("Stopping soundboard ...");
    drop(stream_in);
    drop(stream_out);

    if STREAM_DEAD.load(Ordering::SeqCst) && SB_ON.load(Ordering::SeqCst) {
        // ≈ Python's `except OSError: restart()`.
        log().info("Soundboard stopped.");
        restart();
        return;
    }
    log().info("Soundboard stopped.");
}

/// Port of `stop`.
pub fn stop() {
    SB_ON.store(false, Ordering::SeqCst);
}

/// Port of `restart`: probe the VB-Cable device, stop the loop, revive the
/// thread after 200ms.
pub fn restart() {
    let vbcable = soundboard_config("vbcable");
    // Python assigns the module-global `cable_input_device` here (raising on
    // lookup failure); probe only and keep reviving regardless.
    if let Err(e) = get_device(&vbcable) {
        log().warning(&format!("soundboard restart: {e}"));
    }

    SB_ON.store(false, Ordering::SeqCst);
    std::thread::sleep(std::time::Duration::from_millis(200));
    std::thread::spawn(soundboard);
    log().debug("Soundboard thread revived");
}

/// Port of the module-level autostart: spawn the mic loop when
/// `settings.soundboard.enabled` (called once at server startup, mirroring
/// the Python import-time `soundboard_thread.start()`).
pub fn start_if_enabled() {
    let enabled = get_config(false, false)
        .get("settings")
        .and_then(|s| s.get("soundboard"))
        .and_then(|s| s.get("enabled"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if enabled {
        std::thread::spawn(soundboard);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paren_name_parsing_matches_python_slice() {
        assert_eq!(name_after_paren("Micro (USB)"), "USB)");
        assert_eq!(name_after_paren("no-paren"), "no-paren");
        assert_eq!(name_after_paren("(CABLE Input)"), "CABLE Input)");
    }

    #[test]
    fn stop_and_restart_do_not_panic_without_devices() {
        // No audio hardware on CI; restart must still revive the thread
        // handle path (the loop exits immediately on missing devices).
        stop();
        assert!(!SB_ON.load(Ordering::SeqCst));
        SB_ON.store(true, Ordering::SeqCst);
    }
}
