//! Port of `app/buttons/spotify/songs.py`.

use rspotify::model::{LibraryId, PlayableId, PlayableItem, SearchResult, SearchType};
use rspotify::prelude::*;
use serde_json::{json, Value};

use crate::app::buttons::spotify::utils::{block_on, failure, no_additional_types, SpotifyClient};
use crate::app::utils::logger::log;

/// Port of `save`: save/unsave the currently playing track.
pub fn save(sp: &SpotifyClient) -> Value {
    // Get information about the user's currently playing track.
    let track = match block_on(sp.client.current_playback(None, no_additional_types())) {
        Ok(Some(ctx)) => match ctx.item {
            Some(PlayableItem::Track(track)) => track,
            _ => {
                log().warning("No track currently playing.");
                return failure("spotify_no_track_playing_error".to_string());
            }
        },
        Ok(None) => {
            log().warning("No track currently playing.");
            return failure("spotify_no_track_playing_error".to_string());
        }
        Err(e) => return failure(e.to_string()),
    };

    let track_id = match track.id.clone() {
        Some(id) => id,
        None => {
            log().warning("No track currently playing.");
            return failure("spotify_no_track_playing_error".to_string());
        }
    };
    let artist = track
        .artists
        .first()
        .map(|a| a.name.clone())
        .unwrap_or_else(|| "unknown".to_string());

    match block_on(
        sp.client
            .library_contains([LibraryId::Track(track_id.clone())]),
    ) {
        Ok(flags) => {
            if flags.first().copied().unwrap_or(false) {
                if let Err(e) = block_on(sp.client.library_remove([LibraryId::Track(track_id)])) {
                    return failure(e.to_string());
                }
                log().success(&format!(
                    "Removed track {} by {artist} from saved tracks",
                    track.name
                ));
            } else {
                if let Err(e) = block_on(sp.client.library_add([LibraryId::Track(track_id)])) {
                    return failure(e.to_string());
                }
                log().success(&format!("Saved track {} by {artist}", track.name));
            }
            json!({"success": true})
        }
        Err(e) => failure(e.to_string()),
    }
}

/// Port of `play`: search a track by name and start playback.
pub fn play(sp: &SpotifyClient, song_name: &str) -> Value {
    match block_on(
        sp.client
            .search(song_name, SearchType::Track, None, None, Some(1), Some(0)),
    ) {
        Ok(SearchResult::Tracks(page)) => {
            match page.items.first() {
                Some(track) => match &track.id {
                    Some(id) => {
                        if let Err(e) = block_on(sp.client.start_uris_playback(
                            [PlayableId::Track(id.clone())],
                            None,
                            None,
                            None,
                        )) {
                            return failure(e.to_string());
                        }
                    }
                    None => log().error(&format!("No track found for '{song_name}'")),
                },
                None => log().error(&format!("No track found for '{song_name}'")),
            }
            json!({"success": true})
        }
        Ok(_) => json!({"success": true}),
        Err(e) => failure(e.to_string()),
    }
}
