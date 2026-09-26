//! Port of `app/buttons/spotify/albums.py` — TODO via `rspotify`.

use serde_json::{json, Value};

use crate::app::buttons::spotify::utils::SpotifyClient;
use crate::app::utils::logger::log;

/// Port of `save`.
pub fn save(_sp: &SpotifyClient) -> Value {
    log().warning("spotify album.save: rspotify backend not ported yet");
    json!({"success": false, "message": "spotify backend not ported yet"})
}
