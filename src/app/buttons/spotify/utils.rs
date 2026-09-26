//! Port of `app/buttons/spotify/utils.py`.
//!
//! TODO(port): OAuth + API client via `rspotify` (replacing `spotipy`).

use crate::app::utils::logger::log;

/// Authenticated Spotify client — port of `spotipy.Spotify(auth=token)`.
/// (Holds the token until the `rspotify` backend lands.)
#[derive(Debug, Clone)]
pub struct SpotifyClient {
    pub token: String,
}

/// Port of `initialize` — stub returning no token until OAuth is ported.
pub fn initialize() -> Option<String> {
    log().warning("spotify initialize: rspotify OAuth not ported yet");
    None
}
