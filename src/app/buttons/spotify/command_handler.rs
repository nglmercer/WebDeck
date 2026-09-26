//! Port of `app/buttons/spotify/command_handler.py`.
//!
//! Token caching/refresh flow and subcommand routing are ported 1:1; API
//! calls are TODO via `rspotify`.

use std::sync::{Mutex, OnceLock};

use serde_json::{json, Value};

use crate::app::buttons::spotify::{
    albums::save as save_album,
    artists, playlists, songs,
    utils::{initialize, SpotifyClient},
    volume,
};
use crate::app::utils::{languages::text, logger::log};

/// Port of the module-level `spotify_token` global.
static SPOTIFY_TOKEN: OnceLock<Mutex<String>> = OnceLock::new();

fn token_slot() -> &'static Mutex<String> {
    SPOTIFY_TOKEN.get_or_init(|| Mutex::new(String::new()))
}

/// Port of `handle_command`.
pub fn handle_command(message: &str) -> Value {
    let mut token = token_slot().lock().map(|t| t.clone()).unwrap_or_default();

    if token.is_empty() {
        match initialize() {
            Some(new_token) => {
                token = new_token.clone();
                if let Ok(mut slot) = token_slot().lock() {
                    *slot = new_token;
                }
            }
            None => {
                log().error("Spotify not initialized, check if your credentials are correct.");
                return json!({"success": false, "message": text(Some("spotify_not_initialized"), None)});
            }
        }
    }

    // TODO(port): token-validity check (`current_user`) + re-init on expiry.
    let sp = SpotifyClient { token };

    if message.starts_with("/spotify savesong") || message.starts_with("/spotify likesong") {
        return songs::save(&sp);
    } else if message.starts_with("/spotify savealbum") || message.starts_with("/spotify likealbum") {
        return save_album(&sp);
    } else if message.starts_with("/spotify playsong") {
        let song_name = message.replacen("/spotify playsong", "", 1);
        return songs::play(&sp, song_name.trim());
    } else if message.starts_with("/spotify playplaylist") {
        let playlist_name = message.replacen("/spotify playplaylist", "", 1);
        return playlists::play(&sp, playlist_name.trim());
    } else if message.starts_with("/spotify add_to_playlist")
        || message.starts_with("/spotify remove_from_playlist")
        || message.starts_with("/spotify add_or_remove")
    {
        let playlist_name = message
            .replace("/spotify add_to_playlist", "")
            .replace("/spotify remove_from_playlist", "")
            .replace("/spotify add_or_remove", "");
        return playlists::manage(&sp, message, playlist_name.trim());
    } else if message.starts_with("/spotify follow_artist")
        || message.starts_with("/spotify unfollow_artist")
        || message.starts_with("/spotify follow_or_unfollow_artist")
    {
        return artists::manage(&sp, message);
    } else if message.starts_with("/spotify volume +")
        || message.starts_with("/spotify volume -")
        || message.starts_with("/spotify volume set")
    {
        volume::manage(&sp, message);
    }

    json!({"success": true})
}
