//! Port of `app/buttons/spotify/playlists.py` — TODO via `rspotify`.

use serde_json::{json, Value};

use crate::app::buttons::spotify::utils::SpotifyClient;
use crate::app::utils::logger::log;

/// Port of `play`.
pub fn play(_sp: &SpotifyClient, playlist_name: &str) -> Value {
    log().warning(&format!("spotify playlist.play({playlist_name:?}): rspotify backend not ported yet"));
    json!({"success": false, "message": "spotify backend not ported yet"})
}

/// Port of `manage` (add/remove/add-or-remove).
pub fn manage(_sp: &SpotifyClient, message: &str, playlist_name: &str) -> Value {
    log().warning(&format!(
        "spotify playlist.manage({message:?}, {playlist_name:?}): rspotify backend not ported yet"
    ));
    json!({"success": false, "message": "spotify backend not ported yet"})
}
