//! Port of `app/buttons/spotify/command_handler.py`.

use std::sync::{Mutex, OnceLock};

use rspotify::prelude::OAuthClient;
use serde_json::{json, Value};

use crate::adapters::integrations::spotify::{
    albums::save as save_album,
    artists, playlists, songs,
    utils::{block_on, failure, initialize, SpotifyClient},
    volume,
};
use crate::app::utils::{languages::text, logger::log};

/// Port of the module-level `spotify_token` global (holds the authenticated
/// client instead of the bare token string — see `utils::SpotifyClient`).
static SPOTIFY_CLIENT: OnceLock<Mutex<Option<rspotify::AuthCodeSpotify>>> = OnceLock::new();

fn client_slot() -> &'static Mutex<Option<rspotify::AuthCodeSpotify>> {
    SPOTIFY_CLIENT.get_or_init(|| Mutex::new(None))
}

fn not_initialized() -> Value {
    log().error("Spotify not initialized, check if your credentials are correct.");
    failure(text(Some("spotify_not_initialized"), None))
}

/// Port of `handle_command`.
pub fn handle_command(message: &str) -> Value {
    let mut client = client_slot().lock().ok().and_then(|slot| slot.clone());

    if client.is_none() {
        match initialize() {
            Some(new_client) => {
                if let Ok(mut slot) = client_slot().lock() {
                    *slot = Some(new_client.clone());
                }
                client = Some(new_client);
            }
            None => return not_initialized(),
        }
    }
    let mut client = client.expect("spotify client initialized above");

    // Check if the token is valid by making a simple API call.
    if block_on(client.current_user()).is_err() {
        log().warning("Spotify token expired or invalid, reinitializing.");
        match initialize() {
            Some(new_client) => {
                if let Ok(mut slot) = client_slot().lock() {
                    *slot = Some(new_client.clone());
                }
                client = new_client;
            }
            None => return not_initialized(),
        }
    }

    let sp = SpotifyClient { client };

    if message.starts_with("/spotify savesong") || message.starts_with("/spotify likesong") {
        return songs::save(&sp);
    } else if message.starts_with("/spotify savealbum") || message.starts_with("/spotify likealbum")
    {
        return save_album(&sp);
    } else if message.starts_with("/spotify playsong") {
        let song_name = message.replace("/spotify playsong", "");
        return songs::play(&sp, song_name.trim());
    } else if message.starts_with("/spotify playplaylist") {
        let playlist_name = message.replace("/spotify playplaylist", "");
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
        // Python raises on volume errors (the route maps raises to failure
        // JSON); a clean return maps to `{"success": True}`.
        match volume::manage(&sp, message) {
            Ok(()) => {}
            Err(message) => return failure(message),
        }
    }

    json!({"success": true})
}
