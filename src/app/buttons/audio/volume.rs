//! Port of `app/buttons/audio/volume.py`.
//!
//! Command parsing is ported 1:1; endpoint-volume control is TODO via the
//! `windows` crate CoreAudio APIs (pycaw equivalent).

use crate::app::utils::logger::log;

/// Port of `get_current_volume` — TODO (`windows` CoreAudio).
pub fn get_current_volume() -> Result<f32, String> {
    Err("get_current_volume: CoreAudio backend not ported yet".to_string())
}

/// Port of `set_volume` — TODO (`windows` CoreAudio).
pub fn set_volume(_target_volume: f32) -> Result<f32, String> {
    Err("set_volume: CoreAudio backend not ported yet".to_string())
}

/// Port of `increase_volume`.
pub fn increase_volume(delta: &str) -> Result<f32, String> {
    // TODO(port): VK_VOLUME_UP key event via `windows` crate / enigo.
    log().debug("increase_volume: volume-key event not ported yet");
    if delta.is_empty() {
        return get_current_volume();
    }
    let current = get_current_volume()?;
    let delta: f32 = delta
        .trim()
        .parse::<f32>()
        .map_err(|e| format!("Invalid volume delta {delta:?}: {e}"))?
        / 100.0;
    set_volume(current + delta)
}

/// Port of `decrease_volume`.
pub fn decrease_volume(delta: &str) -> Result<f32, String> {
    // TODO(port): VK_VOLUME_DOWN key event via `windows` crate / enigo.
    log().debug("decrease_volume: volume-key event not ported yet");
    if delta.is_empty() {
        return get_current_volume();
    }
    let current = get_current_volume()?;
    let delta: f32 = delta
        .trim()
        .parse::<f32>()
        .map_err(|e| format!("Invalid volume delta {delta:?}: {e}"))?
        / 100.0;
    set_volume(current - delta)
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
            .parse::<f32>()
        {
            Ok(target) => {
                let _ = set_volume(target / 100.0);
            }
            Err(e) => log().error(&format!("Invalid /volume set target: {e}")),
        }
    }
}
