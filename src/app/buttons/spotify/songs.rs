//! Port of `app/buttons/spotify/songs.py` — TODO via `rspotify`.

use serde_json::{json, Value};

use crate::app::buttons::spotify::utils::SpotifyClient;
use crate::app::utils::logger::log;

/// Port of `save`.
pub fn save(_sp: &SpotifyClient) -> Value {
    log().warning("spotify song.save: rspotify backend not ported yet");
    json!({"success": false, "message": "spotify backend not ported yet"})
}

/// Port of `play`.
pub fn play(_sp: &SpotifyClient, song_name: &str) -> Value {
    log().warning(&format!("spotify song.play({song_name:?}): rspotify backend not ported yet"));
    json!({"success": false, "message": "spotify backend not ported yet"})
}
