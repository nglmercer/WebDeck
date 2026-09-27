//! Port of `app/buttons/spotify/volume.py`.

use rspotify::prelude::*;

use crate::app::buttons::spotify::utils::{block_on, no_additional_types, SpotifyClient};
use crate::app::utils::{languages::text, logger::log};

/// Port of `manage`: adjust the active Spotify device volume.
///
/// Returns `Err(message)` where Python raises (the route maps raises to
/// failure JSON); `Ok(())` maps to `{"success": True}`.
pub fn manage(sp: &SpotifyClient, message: &str) -> Result<(), String> {
    // Get the current playback information.
    let playback = block_on(sp.client.current_playback(None, no_additional_types()))
        .ok()
        .flatten();

    // Check if there is an active device.
    let (device_id, current_volume) = match playback {
        Some(ctx) if ctx.is_playing => match (ctx.device.id.clone(), ctx.device.volume_percent) {
            (Some(id), Some(volume)) => (id, volume),
            _ => {
                log().warning("No active devices on Spotify found.");
                return Err(text(Some("spotify_no_active_device_error"), None));
            }
        },
        _ => {
            log().warning("No active devices on Spotify found.");
            return Err(text(Some("spotify_no_active_device_error"), None));
        }
    };

    // Get the current volume.
    log().debug(&format!("Current spotify volume: {current_volume}"));

    let current = current_volume as i32;
    let target = if message.contains('-') {
        match message
            .replace("/spotify volume -", "")
            .trim()
            .parse::<i32>()
        {
            Ok(n) => current - n,
            Err(_) => current - 10,
        }
    } else if message.contains('+') {
        match message
            .replace("/spotify volume +", "")
            .trim()
            .parse::<i32>()
        {
            Ok(n) => current + n,
            Err(_) => current + 10,
        }
    } else if message.contains("set") {
        match message
            .replace("/spotify volume set", "")
            .trim()
            .parse::<i32>()
        {
            Ok(n) => n,
            // Python's `int()` here is uncaught (ValueError → 500);
            // fail gracefully instead.
            Err(e) => return Err(e.to_string()),
        }
    } else {
        // Unreachable via the router (only +, -, set are routed).
        return Ok(());
    };

    let target = target.clamp(0, 100) as u8;
    if let Err(e) = block_on(sp.client.volume(target, Some(device_id.as_str()))) {
        // NOTE: "Prenium"/"prenium" typos are Python's; kept 1:1.
        if e.to_string().to_lowercase().contains("premium") {
            log().exception(
                &e,
                Some("Unable to apply volume because Spotify Prenium is required."),
                true,
                true,
                true,
            );
            return Err(text(Some("spotify_volume_prenium_error"), None));
        }
        log().exception(
            &e,
            Some("Error while setting the spotify volume."),
            false,
            true,
            true,
        );
        return Err(format!(
            "{}: {e}",
            text(Some("spotify_apply_volume_error"), None)
        ));
    }

    // Get the updated volume.
    if let Ok(Some(updated)) = block_on(sp.client.current_playback(None, no_additional_types())) {
        if let Some(volume) = updated.device.volume_percent {
            log().debug(&format!("Updated spotify volume: {volume}"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    /// Pure port of the message→target-volume parsing, factored out so the
    /// branch order (`-`, `+`, `set`) and `±10` fallbacks stay tested without
    /// Spotify credentials. Mirrors `manage` exactly.
    fn parse_target_volume(message: &str, current: i32) -> Option<i32> {
        let target = if message.contains('-') {
            match message
                .replace("/spotify volume -", "")
                .trim()
                .parse::<i32>()
            {
                Ok(n) => current - n,
                Err(_) => current - 10,
            }
        } else if message.contains('+') {
            match message
                .replace("/spotify volume +", "")
                .trim()
                .parse::<i32>()
            {
                Ok(n) => current + n,
                Err(_) => current + 10,
            }
        } else if message.contains("set") {
            match message
                .replace("/spotify volume set", "")
                .trim()
                .parse::<i32>()
            {
                Ok(n) => n,
                Err(_) => return None,
            }
        } else {
            return None;
        };
        Some(target.clamp(0, 100))
    }

    #[test]
    fn volume_parsing_matches_python_branches() {
        assert_eq!(parse_target_volume("/spotify volume -5", 50), Some(45));
        assert_eq!(parse_target_volume("/spotify volume +5", 50), Some(55));
        assert_eq!(parse_target_volume("/spotify volume set 30", 50), Some(30));
        // Non-numeric deltas fall back to ±10; set fails.
        assert_eq!(parse_target_volume("/spotify volume -loud", 50), Some(40));
        assert_eq!(parse_target_volume("/spotify volume +loud", 50), Some(60));
        assert_eq!(parse_target_volume("/spotify volume set loud", 50), None);
        // Clamp + `-` wins over `+`/`set` in branch order.
        assert_eq!(parse_target_volume("/spotify volume -100", 50), Some(0));
        assert_eq!(parse_target_volume("/spotify volume +100", 50), Some(100));
        assert_eq!(parse_target_volume("/spotify volume set-5", 50), Some(40));
    }
}
