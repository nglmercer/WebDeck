//! Port of `app/utils/settings/audio_devices.py`.
//!
//! TODO(port): enumerate devices via `cpal` (cross-platform PyAudio
//! equivalent), keeping the same filtering: `channels_type` of `"input"` vs
//! anything else (output), `hostApi == 0` equivalent, substring de-dup, and
//! skipping names containing "microsoft - input".

use crate::app::utils::logger::log;

/// Port of `get_audio_devices` — stub returning an empty list until the
/// `cpal` backend lands.
pub fn get_audio_devices(channels_type: &str) -> Vec<String> {
    log().debug(&format!(
        "get_audio_devices({channels_type:?}): cpal backend not ported yet"
    ));
    Vec::new()
}
