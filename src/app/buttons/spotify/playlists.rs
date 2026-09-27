//! Port of `app/buttons/spotify/playlists.py`.

use rspotify::model::{PlayableId, PlayableItem, PlayContextId};
use rspotify::prelude::*;
use serde_json::{json, Value};

use crate::app::buttons::spotify::utils::{block_on, failure, no_additional_types, SpotifyClient};
use crate::app::utils::{languages::text, logger::log};

/// Port of `play`: start playback of the first playlist whose name contains
/// `playlist_name` (first result page only, like spotipy's default).
pub fn play(sp: &SpotifyClient, playlist_name: &str) -> Value {
    let page = match block_on(sp.client.current_user_playlists_manual(None, None)) {
        Ok(page) => page,
        Err(e) => return failure(e.to_string()),
    };

    let wanted = playlist_name.to_lowercase();
    let wanted = wanted.trim();
    for playlist in &page.items {
        if playlist.name.to_lowercase().trim().contains(wanted) {
            if let Err(e) = block_on(sp.client.start_context_playback(
                PlayContextId::Playlist(playlist.id.clone()),
                None,
                None,
                None,
            )) {
                return failure(e.to_string());
            }
            return json!({"success": true});
        }
    }

    log().error(&format!("Playlist '{playlist_name}' not found."));
    json!({"success": true})
}

/// Port of `manage`: add/remove/toggle the current track in a playlist.
pub fn manage(sp: &SpotifyClient, message: &str, playlist_name: &str) -> Value {
    let page = match block_on(sp.client.current_user_playlists_manual(None, None)) {
        Ok(page) => page,
        Err(e) => return failure(e.to_string()),
    };
    let playlist_id = match page.items.iter().find(|p| p.name == playlist_name) {
        Some(playlist) => playlist.id.clone(),
        None => {
            return failure(
                text(Some("spotify_playlist_not_found_error"), None)
                    .replace("%playlist_name%", playlist_name),
            );
        }
    };

    let track = match block_on(sp.client.current_playback(None, no_additional_types())) {
        Ok(Some(ctx)) => match ctx.item {
            Some(PlayableItem::Track(track)) => track,
            _ => return failure("spotify_no_track_playing_error".to_string()),
        },
        // Python raises TypeError here; fail gracefully.
        _ => return failure("spotify_no_track_playing_error".to_string()),
    };
    let track_id = match track.id.clone() {
        Some(id) => id,
        None => return failure("spotify_no_track_playing_error".to_string()),
    };
    let track_uri = track_id.uri();

    if message.contains("add_or_remove") || message.contains("toggle") {
        let items = match block_on(sp.client.playlist_items_manual(
            playlist_id.clone(),
            Some("items(track(uri))"),
            None,
            None,
            None,
        )) {
            Ok(page) => page,
            Err(e) => return failure(e.to_string()),
        };
        let track_uris: Vec<String> = items
            .items
            .iter()
            .filter_map(|item| item.item.as_ref())
            .filter_map(|item| match item {
                PlayableItem::Track(track) => track.id.as_ref().map(|id| id.uri()),
                _ => None,
            })
            .collect();

        if track_uris.contains(&track_uri) {
            if let Err(e) = block_on(sp.client.playlist_remove_all_occurrences_of_items(
                playlist_id,
                [PlayableId::Track(track_id)],
                None,
            )) {
                return failure(e.to_string());
            }
            log().success("The track has been removed from the playlist.");
        } else {
            if let Err(e) = block_on(sp.client.playlist_add_items(
                playlist_id,
                [PlayableId::Track(track_id)],
                None,
            )) {
                return failure(e.to_string());
            }
            log().success("The track has been added to the playlist.");
        }
    } else if message.contains("add_to_playlist") {
        if let Err(e) = block_on(sp.client.playlist_add_items(
            playlist_id,
            [PlayableId::Track(track_id)],
            None,
        )) {
            return failure(e.to_string());
        }
        log().success("The track has been added to the playlist.");
    } else if message.contains("remove_from_playlist") {
        if let Err(e) = block_on(sp.client.playlist_remove_all_occurrences_of_items(
            playlist_id,
            [PlayableId::Track(track_id)],
            None,
        )) {
            return failure(e.to_string());
        }
        log().success("The track has been removed from the playlist.");
    }

    json!({"success": true})
}
