//! Port of `app/buttons/spotify/artists.py` — TODO via `rspotify`.

use serde_json::{json, Value};

use crate::app::buttons::spotify::utils::SpotifyClient;
use crate::app::utils::logger::log;

/// Port of `manage` (follow/unfollow/follow-or-unfollow).
pub fn manage(_sp: &SpotifyClient, message: &str) -> Value {
    log().warning(&format!("spotify artist.manage({message:?}): rspotify backend not ported yet"));
    json!({"success": false, "message": "spotify backend not ported yet"})
}
