//! Port of `app/buttons/spotify/albums.py`.

use rspotify::model::{LibraryId, PlayableItem};
use rspotify::prelude::*;
use serde_json::{json, Value};

use crate::app::buttons::spotify::utils::{block_on, failure, no_additional_types, SpotifyClient};
use crate::app::utils::logger::log;

/// Port of `save`: save/unsave the currently playing album.
pub fn save(sp: &SpotifyClient) -> Value {
    // Get information about the user's currently playing track.
    let album = match block_on(sp.client.current_playback(None, no_additional_types())) {
        Ok(Some(ctx)) => match ctx.item {
            Some(PlayableItem::Track(track)) => track.album,
            _ => {
                log().notice("No album currently playing.");
                return json!({"success": true});
            }
        },
        Ok(None) => {
            log().notice("No album currently playing.");
            return json!({"success": true});
        }
        Err(e) => return failure(e.to_string()),
    };

    let album_id = match album.id.clone() {
        Some(id) => id,
        None => {
            log().notice("No album currently playing.");
            return json!({"success": true});
        }
    };
    let artists = album
        .artists
        .iter()
        .map(|a| a.name.clone())
        .collect::<Vec<_>>()
        .join(", ");

    match block_on(
        sp.client
            .library_contains([LibraryId::Album(album_id.clone())]),
    ) {
        Ok(flags) => {
            if flags.first().copied().unwrap_or(false) {
                if let Err(e) = block_on(
                    sp.client
                        .library_remove([LibraryId::Album(album_id)]),
                ) {
                    return failure(e.to_string());
                }
                log().info(&format!(
                    "Removed album '{}' by {artists} from saved albums",
                    album.name
                ));
            } else {
                if let Err(e) =
                    block_on(sp.client.library_add([LibraryId::Album(album_id)]))
                {
                    return failure(e.to_string());
                }
                log().info(&format!("Saved album '{}' by {artists}", album.name));
            }
            json!({"success": true})
        }
        Err(e) => failure(e.to_string()),
    }
}
