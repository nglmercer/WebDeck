//! Port of `app/buttons/soundboard/ffmpeg.py`.
//!
//! [`replace_last_element`] is ported 1:1 (pure string helper). The rest
//! (FFmpeg download/install, silence padding, mp3→wav via `pydub`) is TODO —
//! planned via `ffmpeg-sidecar` + `symphonia`.

use crate::app::utils::logger::log;

/// Port of `install_ffmpeg` — stub.
pub fn install_ffmpeg() -> Option<String> {
    log().warning("soundboard install_ffmpeg: ffmpeg-sidecar backend not ported yet");
    None
}

/// Port of `get_ffmpeg` — stub.
pub fn get_ffmpeg() -> Option<String> {
    log().debug("soundboard get_ffmpeg: ffmpeg-sidecar backend not ported yet");
    None
}

/// Port of `replace_last_element`.
pub fn replace_last_element(string: &str, old_element: &str, new_element: &str) -> String {
    match string.rfind(old_element) {
        Some(last_index) => {
            format!(
                "{}{}{}",
                &string[..last_index],
                &string[last_index..].replacen(old_element, new_element, 1),
                ""
            )
        }
        None => string.to_string(),
    }
}

/// Port of `add_silence_to_end` — stub.
pub fn add_silence_to_end(_input_file: &str, _output_file: &str, _silence_duration_ms: u64) -> bool {
    log().warning("soundboard add_silence_to_end: audio backend not ported yet");
    false
}

/// Port of `silence_path` — stub.
pub fn silence_path(input_file: &str, _remove_previous: bool) -> String {
    log().warning("soundboard silence_path: audio backend not ported yet");
    input_file.to_string()
}

/// Port of `to_wav` — stub.
pub fn to_wav(input_file: &str, output_file: Option<&str>, _volume: f32) -> Option<String> {
    log().warning("soundboard to_wav: audio backend not ported yet");
    output_file.map(|s| s.to_string()).or_else(|| Some(input_file.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replaces_only_last_occurrence() {
        assert_eq!(replace_last_element("a-b-a", "a", "z"), "a-b-z");
        assert_eq!(replace_last_element("aaa", "a", "b"), "aab");
        assert_eq!(replace_last_element("abc", "z", "q"), "abc");
    }
}
