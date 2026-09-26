//! Port of `app/buttons/soundboard/devices.py`.
//!
//! TODO(port): output-device enumeration via VLC bindings (`vlc` crate) or
//! `cpal`, replacing `vlc.MediaPlayer().audio_output_device_enum()`.

use crate::app::utils::logger::log;

/// Port of `get_device` — stub.
pub fn get_device(device: &str) -> Result<String, String> {
    log().warning(&format!(
        "soundboard get_device({device:?}): VLC backend not ported yet"
    ));
    Err(format!(
        "Failed to retrieve audio device '{device}'. Ensure VLC is installed properly."
    ))
}
