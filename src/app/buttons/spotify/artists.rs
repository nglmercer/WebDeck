//! Port of `app/buttons/spotify/artists.py`.

use rspotify::model::{LibraryId, PlayableItem};
use rspotify::prelude::*;
use serde_json::{json, Value};

use crate::app::buttons::spotify::utils::{block_on, failure, no_additional_types, SpotifyClient};
use crate::app::utils::{languages::text, logger::log};

/// Port of `manage`: follow/unfollow/toggle the current track's artist.
pub fn manage(sp: &SpotifyClient, message: &str) -> Value {
    let playback = match block_on(sp.client.current_playback(None, no_additional_types())) {
        Ok(Some(ctx)) => ctx,
        // Python raises TypeError here (`None["item"]`); fail gracefully.
        _ => return failure("spotify_no_track_playing_error".to_string()),
    };
    let artist = match &playback.item {
        Some(PlayableItem::Track(track)) => match track.artists.first() {
            Some(artist) => artist,
            None => return failure("spotify_no_track_playing_error".to_string()),
        },
        _ => return failure("spotify_no_track_playing_error".to_string()),
    };
    let artist_id = match artist.id.clone() {
        Some(id) => id,
        None => return failure("spotify_no_track_playing_error".to_string()),
    };
    let artist_name = artist.name.clone();

    if message.contains("follow_or_unfollow_artist") || message.contains("toggle_follow") {
        match block_on(sp.client.library_contains([LibraryId::Artist(artist_id.clone())])) {
            Ok(flags) => {
                if flags.first().copied().unwrap_or(false) {
                    log().debug(&format!(
                        "The user is subscribed to the artist '{artist_name}'."
                    ));
                    return unfollow_artist(sp, &artist_id, &artist_name);
                }
                log().debug(&format!(
                    "The user is not subscribed to the artist '{artist_name}'."
                ));
                return follow_artist(sp, &artist_id, &artist_name);
            }
            Err(e) => return failure(e.to_string()),
        }
    } else if message.contains("unfollow_artist") {
        return unfollow_artist(sp, &artist_id, &artist_name);
    } else if message.contains("follow_artist") {
        return follow_artist(sp, &artist_id, &artist_name);
    }

    json!({"success": true})
}

fn follow_artist(
    sp: &SpotifyClient,
    artist_id: &rspotify::model::ArtistId,
    artist_name: &str,
) -> Value {
    if let Err(e) = block_on(sp.client.library_add([LibraryId::Artist(artist_id.clone())])) {
        return failure(e.to_string());
    }
    log().success("The artist has been added to the subscription list.");
    json!({
        "success": true,
        "message": text(Some("spotify_follow_artist_success"), None).replace("%artist_name%", artist_name),
    })
}

fn unfollow_artist(
    sp: &SpotifyClient,
    artist_id: &rspotify::model::ArtistId,
    artist_name: &str,
) -> Value {
    if let Err(e) = block_on(sp.client.library_remove([LibraryId::Artist(artist_id.clone())])) {
        return failure(e.to_string());
    }
    log().success("The artist has been removed from the subscription list.");
    json!({
        "success": true,
        "message": text(Some("spotify_unfollow_artist_success"), None).replace("%artist_name%", artist_name),
    })
}
