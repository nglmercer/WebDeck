//! Port of `app/buttons/spotify/volume.py` — TODO via `rspotify`.

use crate::app::buttons::spotify::utils::SpotifyClient;
use crate::app::utils::logger::log;

/// Port of `manage`.
pub fn manage(_sp: &SpotifyClient, message: &str) {
    log().warning(&format!("spotify volume.manage({message:?}): rspotify backend not ported yet"));
}
