//! Port of `app/buttons/soundboard/player.py`.
//!
//! Backend mapping: `python-vlc` / `nava` → [`rodio`] 0.22
//! (`DeviceSinkBuilder` + `Player`, mp3/flac/vorbis/wav via symphonia).
//! Slot allocation (`p_id = max(len, len)`, 4 slots, else restart slot 0),
//! the VB-Cable + local-monitor pairing, the `fix_stop_soundboard` silence
//! padding, and the `stopsound` gates are all 1:1 with Python.
//!
//! Documented deviations (mechanical, no behavior lost):
//! - Python tracks `player_vbcable` / `player_local` dicts; Rust tracks one
//!   map of slots holding both sides (same keys, same pairing).
//! - `MediaPlayerEndReached` events → a 500ms reaper thread that drops
//!   finished slots (same observable cleanup).
//! - nava plays (untracked in Python, stopped via `nava.stop_all`) live in a
//!   second map so `stopsound` still stops them first, as in Python.
//! - Where Python raises (`RuntimeError`, `KeyError`, view returning `None`)
//!   this returns a failure JSON object instead of a Flask 500.

use std::collections::BTreeMap;
use std::fs::File;
use std::io::BufReader;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};

use rodio::cpal::traits::{DeviceTrait, HostTrait};
use rodio::{Decoder, DeviceSinkBuilder, MixerDeviceSink, Player};
use serde_json::{json, Value};

use super::devices::get_device;
use super::ffmpeg::{silence_path, to_wav};
use crate::app::utils::languages::text;
use crate::app::utils::{logger::log, settings::get_config::get_config};

/// One live playback. The rodio stream must stay alive alongside its player,
/// and the source file + volume are retained so slot 0 can be restarted
/// exactly like Python's `stop()` / `set_time(0)` / `play()`.
struct SlotSink {
    _stream: MixerDeviceSink,
    player: Player,
    file: String,
    volume: f32,
}

/// One soundboard slot: VB-Cable feed + local monitor — the paired
/// `player_vbcable[p_id]` / `player_local[p_id]` entries.
#[derive(Default)]
struct Slot {
    vbcable: Option<SlotSink>,
    local: Option<SlotSink>,
}

fn players() -> &'static Mutex<BTreeMap<u64, Slot>> {
    static PLAYERS: OnceLock<Mutex<BTreeMap<u64, Slot>>> = OnceLock::new();
    PLAYERS.get_or_init(|| Mutex::new(BTreeMap::new()))
}

/// nava-method plays (default device only, like `nava.play`).
fn nava_players() -> &'static Mutex<BTreeMap<u64, SlotSink>> {
    static NAVA: OnceLock<Mutex<BTreeMap<u64, SlotSink>>> = OnceLock::new();
    NAVA.get_or_init(|| Mutex::new(BTreeMap::new()))
}

fn lock_players() -> Option<MutexGuard<'static, BTreeMap<u64, Slot>>> {
    players().lock().ok()
}

fn lock_nava() -> Option<MutexGuard<'static, BTreeMap<u64, SlotSink>>> {
    nava_players().lock().ok()
}

/// Start the finished-slot reaper once (≈ VLC `MediaPlayerEndReached`).
fn ensure_reaper() {
    static STARTED: AtomicBool = AtomicBool::new(false);
    if STARTED.swap(true, Ordering::SeqCst) {
        return;
    }
    std::thread::spawn(|| loop {
        std::thread::sleep(std::time::Duration::from_millis(500));
        if let Some(mut map) = lock_players() {
            map.retain(|_, slot| {
                if slot.vbcable.as_ref().is_some_and(|s| s.player.empty()) {
                    slot.vbcable = None;
                }
                if slot.local.as_ref().is_some_and(|s| s.player.empty()) {
                    slot.local = None;
                }
                slot.vbcable.is_some() || slot.local.is_some()
            });
        }
        if let Some(mut map) = lock_nava() {
            map.retain(|_, sink| !sink.player.empty());
        }
    });
}

/// Resolve a live output device by the exact name [`get_device`] returned.
fn find_output_device(name: &str) -> Option<rodio::Device> {
    let host = rodio::cpal::default_host();
    let devices = host.output_devices().ok()?;
    for device in devices {
        if device.description().ok().is_some_and(|d| d.name() == name) {
            return Some(device);
        }
    }
    None
}

/// Open a rodio stream on `device_name` (`None` = default device) and start
/// `file_path` at `volume` (0.0–1.0, like VLC's `sound_volume * 100`).
fn open_sink(device_name: Option<&str>, file_path: &str, volume: f32) -> Option<SlotSink> {
    let builder = match device_name {
        Some(name) => DeviceSinkBuilder::from_device(find_output_device(name)?).ok()?,
        None => DeviceSinkBuilder::from_default_device().ok()?,
    };
    let stream = builder.open_stream().ok()?;
    let player = Player::connect_new(stream.mixer());
    player.set_volume(volume);
    let file = File::open(file_path).ok()?;
    let decoder = Decoder::new(BufReader::new(file)).ok()?;
    player.append(decoder);
    Some(SlotSink {
        _stream: stream,
        player,
        file: file_path.to_string(),
        volume,
    })
}

fn soundboard_setting(config: &Value, key: &str) -> Option<String> {
    config
        .get("settings")
        .and_then(|s| s.get("soundboard"))
        .and_then(|s| s.get(key))
        .and_then(|v| v.as_str())
        .map(str::to_string)
}

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
        .replace("C:\\\\fakepath\\\\", "")
        .replace("/playsound ", "")
        .replace("/playlocalsound ", "");
    // Python: `message[message.rfind(" ") + 1:]` — no space means the whole
    // message is the percentage candidate.
    let percentage = match message.rfind(' ') {
        Some(i) => message[i + 1..].replace(' ', ""),
        None => message.clone(),
    };

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
    if ![":", ".config/user_uploads/", ".config\\\\user_uploads\\\\"]
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

/// Port of `playsound_nava`: default output device only, volume baked into
/// the converted WAV by [`to_wav`] (nava itself has no volume parameter, so
/// the rodio player runs at 1.0 to avoid double-scaling).
pub fn playsound_nava(
    file_path: &str,
    sound_volume: f32,
    _ear_soundboard: bool,
    _localonly: bool,
) -> Value {
    let wav = match to_wav(file_path, None, sound_volume) {
        Some(path) => path,
        None => return json!({"success": false, "message": "could not convert sound to wav"}),
    };
    ensure_reaper();
    match open_sink(None, &wav, 1.0) {
        Some(sink) => {
            if let Some(mut map) = lock_nava() {
                let p_id = map.len() as u64;
                map.insert(p_id, sink);
            }
            json!({"success": true})
        }
        None => json!({"success": false, "message": "could not play sound"}),
    }
}

/// Port of `playsound_vlc`.
pub fn playsound_vlc(
    file_path: &str,
    sound_volume: f32,
    ear_soundboard: bool,
    localonly: bool,
) -> Value {
    let config = get_config(false, false);
    let vbcable_name = soundboard_setting(&config, "vbcable").unwrap_or_default();
    let cable_device = match get_device(&vbcable_name) {
        Ok(device) => device,
        // Python raises RuntimeError here (Flask 500); fail gracefully.
        Err(e) => return json!({"success": false, "message": e}),
    };

    let file_path = if config
        .get("settings")
        .and_then(|s| s.get("fix_stop_soundboard"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        match silence_path(file_path, false) {
            Some(path) => path,
            None => {
                log().error("FFmpeg is not installed!");
                return json!({"success": false, "message": text(Some("ffmpeg_not_installed_error"), None)});
            }
        }
    } else {
        file_path.to_string()
    };

    ensure_reaper();
    let Some(mut map) = lock_players() else {
        return json!({"success": false, "message": "soundboard busy"});
    };

    let vbcable_keys: Vec<u64> = map
        .iter()
        .filter(|(_, slot)| slot.vbcable.is_some())
        .map(|(id, _)| *id)
        .collect();
    let local_keys: Vec<u64> = map
        .iter()
        .filter(|(_, slot)| slot.local.is_some())
        .map(|(id, _)| *id)
        .collect();
    log().debug(&format!("Current VLC player for vbcable: {vbcable_keys:?}"));
    log().debug(&format!("Current VLC player for local: {local_keys:?}"));

    // Python: `p_id = max(len(...), len(...))` — lengths, not max keys.
    let p_id = vbcable_keys.len().max(local_keys.len()) as u64;

    if p_id <= 3 {
        let mut slot = Slot::default();
        if !localonly {
            if let Some(sink) = open_sink(Some(&cable_device), &file_path, sound_volume) {
                slot.vbcable = Some(sink);
            }
        }
        if ear_soundboard || localonly {
            if let Some(sink) = open_sink(None, &file_path, sound_volume) {
                slot.local = Some(sink);
            }
        }
        map.insert(p_id, slot);
    } else {
        // Restart slot 0 with its own (old) file, like Python's
        // `stop()` / `set_time(0)` / `play()` (the new file is ignored,
        // exactly as in Python).
        let entry = map.entry(0).or_default();
        if !localonly {
            restart_side(entry.vbcable.as_ref());
            if entry.vbcable.is_none() {
                // Python would KeyError here; start fresh instead.
                if let Some(fresh) = open_sink(Some(&cable_device), &file_path, sound_volume) {
                    entry.vbcable = Some(fresh);
                }
            }
        }
        if ear_soundboard || localonly {
            restart_side(entry.local.as_ref());
            if entry.local.is_none() {
                if let Some(fresh) = open_sink(None, &file_path, sound_volume) {
                    entry.local = Some(fresh);
                }
            }
        }
    }

    log().success(&format!(
        "Playing sound from file: {file_path} at volume: {}%",
        sound_volume * 100.0
    ));
    json!({"success": true})
}

/// Stop a slot-0 side and replay its own file from the start.
fn restart_side(side: Option<&SlotSink>) {
    let Some(sink) = side else {
        return;
    };
    sink.player.stop();
    if let Ok(file) = File::open(&sink.file) {
        if let Ok(decoder) = Decoder::new(BufReader::new(file)) {
            sink.player.append(decoder);
        }
    }
    sink.player.set_volume(sink.volume);
    sink.player.play();
}

/// Port of `stopsound`.
pub fn stopsound() -> Value {
    // NAVA (`nava.stop_all()`): always, before the device gate.
    if let Some(mut nava) = lock_nava() {
        for (_, sink) in nava.iter() {
            sink.player.stop();
        }
        nava.clear();
    }

    let config = get_config(false, false);
    let vbcable_name = soundboard_setting(&config, "vbcable").unwrap_or_default();
    // Python raises here when enumeration fails, and falls off the end
    // (Flask 500) when the device is simply missing; fail gracefully.
    if get_device(&vbcable_name).is_err() {
        return json!({"success": false, "message": format!("Failed to retrieve audio device '{vbcable_name}'.")});
    }

    let Some(mut map) = lock_players() else {
        return json!({"success": true});
    };
    if map.is_empty() {
        log().notice("There are no sounds actually playing");
        let method = config
            .get("settings")
            .and_then(|s| s.get("soundboard"))
            .and_then(|s| s.get("audio_method"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if method != "nava" {
            return json!({"success": true, "message": "There are no sounds actually playing"});
        }
        return json!({"success": true});
    }

    // Python only stops when the LAST player is still Playing
    // (`while str(last.get_state()) == "State.Playing"`).
    let last_playing = map.iter().next_back().is_some_and(|(_, slot)| {
        slot.vbcable.as_ref().is_some_and(|s| !s.player.empty())
            || slot.local.as_ref().is_some_and(|s| !s.player.empty())
    });
    if last_playing {
        for (_, slot) in map.iter() {
            if let Some(sink) = &slot.vbcable {
                sink.player.stop();
            }
            if let Some(sink) = &slot.local {
                sink.player.stop();
            }
        }
    }
    map.clear();

    log().success("Soundboard: All sounds stopped");
    json!({"success": true})
}

/// Port of `remove_player`: drop one side of a slot (1 = vbcable, 2 =
/// local), dropping the slot itself when both sides are gone. Unknown
/// `sb_type` values and missing ids are no-ops, like Python's
/// `del p_id` / `except KeyError: pass`.
pub fn remove_player(sb_type: u8, p_id: u64) {
    if let Some(mut map) = lock_players() {
        let drop_slot = match sb_type {
            1 => {
                if let Some(slot) = map.get_mut(&p_id) {
                    slot.vbcable = None;
                    slot.local.is_none()
                } else {
                    false
                }
            }
            2 => {
                if let Some(slot) = map.get_mut(&p_id) {
                    slot.local = None;
                    slot.vbcable.is_none()
                } else {
                    false
                }
            }
            _ => false,
        };
        if drop_slot {
            map.remove(&p_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn params_parse_volume_and_prefix_upload_dir() {
        // /playlocalsound forces localonly + ear without touching config.
        // (The trailing space is Python's too: the percentage is replaced,
        // not the " <percentage>" suffix.)
        let params = get_params("/playlocalsound mysound 75");
        assert_eq!(params.file_path, ".config/user_uploads/mysound ");
        assert!((params.sound_volume - 0.75).abs() < f32::EPSILON);
        assert!(params.localonly);
        assert!(params.ear_soundboard);
    }

    #[test]
    fn params_default_volume_when_not_numeric() {
        let params = get_params("/playlocalsound mysound loud");
        assert_eq!(params.file_path, ".config/user_uploads/mysound loud");
        assert!((params.sound_volume - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn params_keep_absolute_and_nospace_forms_like_python() {
        // No-space numeric message: Python treats the whole message as the
        // percentage, leaving an empty file name → bare upload prefix.
        let params = get_params("/playlocalsound 60");
        assert_eq!(params.file_path, ".config/user_uploads/");
        assert!((params.sound_volume - 0.6).abs() < f32::EPSILON);

        let params = get_params("/playlocalsound C:\\music\\x.mp3 50");
        assert_eq!(params.file_path, "C:\\music\\x.mp3 ");
    }

    #[test]
    fn remove_player_with_unknown_ids_is_noop() {
        remove_player(1, 999_999);
        remove_player(9, 0);
    }
}
