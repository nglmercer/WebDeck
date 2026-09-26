//! Port of `app/buttons/soundboard/player.py`.
//!
//! [`get_params`] and the `playsound` vlc/nava dispatch are ported 1:1;
//! playback itself is TODO (`vlc` crate / `rodio`, replacing
//! `python-vlc`/`nava`).

use serde_json::{json, Value};

use crate::app::utils::{logger::log, settings::get_config::get_config};

/// Parsed `/playsound` arguments — port of the `get_params` return tuple.
#[derive(Debug, Clone)]
pub struct SoundParams {
    pub file_path: String,
    pub sound_volume: f32,
    pub ear_soundboard: bool,
    pub localonly: bool,
}

/// Port of `get_params`.
pub fn get_params(msg: &str) -> SoundParams {
    let message = msg
        .replace("C:\\fakepath\\", "")
        .replace("/playsound ", "")
        .replace("/playlocalsound ", "");
    let percentage = message
        .rfind(' ')
        .map(|i| message[i + 1..].replace(' ', ""))
        .unwrap_or_default();

    let (sound_file, sound_volume) = match percentage.parse::<f32>() {
        Ok(volume) => (
            message
                .replace("/playsound ", "")
                .replace("/playlocalsound ", "")
                .replace(&percentage, ""),
            volume / 100.0,
        ),
        Err(_) => (
            message
                .replace("/playsound ", "")
                .replace("/playlocalsound ", ""),
            50.0 / 100.0, // mid volume (default)
        ),
    };

    let mut sound_file = sound_file;
    if ![":", ".config/user_uploads/", ".config\\user_uploads\\"]
        .iter()
        .any(|substring| sound_file.contains(substring))
    {
        // Stored directly in .config/user_uploads, not an absolute path.
        sound_file = format!(".config/user_uploads/{sound_file}");
    }

    let (localonly, ear_soundboard) = if msg.starts_with("/playlocalsound") {
        (true, true)
    } else {
        let config = get_config(false, false);
        let ear = config
            .get("settings")
            .and_then(|s| s.get("ear_soundboard"))
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        (false, ear)
    };

    SoundParams {
        file_path: sound_file,
        sound_volume,
        ear_soundboard,
        localonly,
    }
}

/// Port of `playsound` — dispatches on `settings.soundboard.audio_method`.
pub fn playsound(
    file_path: &str,
    sound_volume: f32,
    ear_soundboard: bool,
    localonly: bool,
) -> Value {
    let config = get_config(false, false);
    let method = config
        .get("settings")
        .and_then(|s| s.get("soundboard"))
        .and_then(|s| s.get("audio_method"))
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if method == "vlc" {
        return playsound_vlc(file_path, sound_volume, ear_soundboard, localonly);
    } else if method == "nava" {
        return playsound_nava(file_path, sound_volume, ear_soundboard, localonly);
    }

    json!({"success": false, "message": format!("Unknown soundboard audio_method: {method}")})
}

/// Port of `playsound_nava` — stub.
pub fn playsound_nava(
    file_path: &str,
    sound_volume: f32,
    _ear_soundboard: bool,
    _localonly: bool,
) -> Value {
    log().warning(&format!(
        "playsound_nava({file_path:?}, volume={sound_volume}): audio backend not ported yet"
    ));
    json!({"success": false, "message": "soundboard playback not ported yet"})
}

/// Port of `playsound_vlc` — stub.
pub fn playsound_vlc(
    file_path: &str,
    sound_volume: f32,
    _ear_soundboard: bool,
    _localonly: bool,
) -> Value {
    log().warning(&format!(
        "playsound_vlc({file_path:?}, volume={sound_volume}): VLC backend not ported yet"
    ));
    json!({"success": false, "message": "soundboard playback not ported yet"})
}

/// Port of `stopsound` — stub.
pub fn stopsound() -> Value {
    log().debug("stopsound (audio backend not ported yet)");
    json!({"success": true})
}

/// Port of `remove_player` — stub.
pub fn remove_player(sb_type: &str, p_id: &str) {
    log().debug(&format!(
        "remove_player({sb_type:?}, {p_id:?}) (audio backend not ported yet)"
    ));
}
