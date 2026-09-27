//! Port of `app/buttons/spotify/utils.py`.
//!
//! Backend mapping: `spotipy` → [`rspotify`] (`AuthCodeSpotify`, auth-code
//! flow with the same `http://localhost:8888/callback` redirect and the
//! same 11 scopes). The token cache lives at `.cache-<username>` like
//! spotipy's (rspotify JSON format, so the first run re-authenticates).
//!
//! `handle_command` runs on sync `spawn_blocking` threads, so every API
//! call goes through [`block_on`] (a throwaway current-thread runtime per
//! call — no ambient runtime there, and the ~µs build cost is negligible
//! next to the HTTPS round trip).

use rspotify::prelude::OAuthClient;
use rspotify::{scopes, AuthCodeSpotify, Config, Credentials, OAuth};
use serde_json::{json, Value};

use crate::app::utils::{logger::log, settings::get_config::get_config};

/// Run an async Spotify call from sync command context (mirrors spotipy's
/// blocking API). Safe from `spawn_blocking` threads (no ambient runtime).
pub fn block_on<F: std::future::Future>(future: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("spotify: cannot build tokio runtime")
        .block_on(future)
}

/// `current_playback` extra-types argument. Python passes no
/// `additional_types`; the opaque-iterator type just needs pinning.
pub fn no_additional_types() -> Option<std::iter::Empty<&'static rspotify::model::AdditionalType>> {
    None
}

/// Authenticated Spotify client — port of `spotipy.Spotify(auth=token)`.
///
/// Python rebuilds the client from the cached token *string* on every
/// command; Rust keeps the authenticated `AuthCodeSpotify` itself (same
/// observable behavior, plus free token refresh from the cached refresh
/// token instead of a full re-login on expiry).
#[derive(Debug, Clone)]
pub struct SpotifyClient {
    pub client: AuthCodeSpotify,
}

/// Port of the Flask route's exception mapping: a raise with message `m`
/// becomes `{"success": false, "message": m}` (`send_data_route` catches).
pub fn failure(message: String) -> Value {
    json!({"success": false, "message": message})
}

/// Port of `initialize`: authenticate (cached token or browser flow) and
/// return the client, or `None` when credentials are missing / auth fails.
pub fn initialize() -> Option<AuthCodeSpotify> {
    // Reload config before assuming it's not set (Python re-reads the file).
    let config = get_config(true, false);
    let api = config.get("settings")?.get("spotify_api")?.clone();
    let username = api.get("username").and_then(|v| v.as_str()).unwrap_or("");
    let client_id = api.get("client_id").and_then(|v| v.as_str()).unwrap_or("");
    let client_secret = api
        .get("client_secret")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    // Check if client id and client secret are set in the config.
    if client_id.is_empty() || client_secret.is_empty() {
        log().warning("Spotify client ID and/or client secret not set in the config.");
        return None;
    }

    // Set up the Spotify API client.
    let result = (|| -> Result<AuthCodeSpotify, rspotify::ClientError> {
        let creds = Credentials::new(client_id, client_secret);
        let oauth = OAuth {
            redirect_uri: "http://localhost:8888/callback".to_string(),
            scopes: scopes!(
                "user-library-modify",
                "user-library-read",
                "user-read-currently-playing",
                "user-read-playback-state",
                "user-modify-playback-state",
                "playlist-read-private",
                "playlist-read-collaborative",
                "playlist-modify-private",
                "playlist-modify-public",
                "user-follow-modify",
                "user-follow-read"
            ),
            ..Default::default()
        };
        let config = Config {
            token_cached: true,
            cache_path: std::path::PathBuf::from(format!(".cache-{username}")),
            ..Default::default()
        };
        let spotify = AuthCodeSpotify::with_config(creds, oauth, config);
        // Port of `util.prompt_for_user_token(...)`: read the token cache
        // (refreshing when expired), else run the browser auth flow.
        let url = spotify.get_authorize_url(false)?;
        block_on(spotify.prompt_for_token(&url))?;
        Ok(spotify)
    })();

    match result {
        Ok(client) => Some(client),
        Err(e) => {
            log().exception(&e, Some("Failed to start spotipy"), true, true, true);
            None
        }
    }
}
