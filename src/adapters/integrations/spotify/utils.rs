//! Port of `app/buttons/spotify/utils.py`.
//!
//! Backend mapping: `spotipy` → [`rspotify`] (`AuthCodeSpotify`, auth-code
//! flow with the same `http://localhost:8888/callback` redirect and the
//! same 11 scopes). A private token cache is stored under the configured data directory.
//!
//! `handle_command` runs on sync `spawn_blocking` threads, so every API
//! call goes through [`block_on`] (a throwaway current-thread runtime per
//! call — no ambient runtime there, and the ~µs build cost is negligible
//! next to the HTTPS round trip).

use rspotify::prelude::{BaseClient, OAuthClient};
use rspotify::{scopes, AuthCodeSpotify, Config, Credentials, OAuth};
use serde_json::{json, Value};

use crate::app::utils::{logger::log, settings::get_config::get_config};

/// Run an async Spotify call from sync command context (mirrors spotipy's
/// blocking API). Safe from `spawn_blocking` threads (no ambient runtime).
pub fn block_on<T, F: std::future::Future<Output = Result<T, rspotify::ClientError>>>(
    future: F,
) -> Result<T, rspotify::ClientError> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("spotify: cannot build tokio runtime")
        .block_on(async {
            tokio::time::timeout(std::time::Duration::from_secs(30), future)
                .await
                .unwrap_or_else(|_| {
                    Err(rspotify::ClientError::Io(std::io::Error::new(
                        std::io::ErrorKind::TimedOut,
                        "Spotify operation timed out",
                    )))
                })
        })
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
            cache_path: crate::app::utils::settings::get_config::config_dir().join(format!(
                "spotify-{}.json",
                crate::adapters::update::digest(username.as_bytes())
            )),
            ..Default::default()
        };
        let spotify = AuthCodeSpotify::with_config(creds, oauth, config);
        // Port of `util.prompt_for_user_token(...)`: read the token cache
        // (refreshing when expired), else run the browser auth flow.
        let url = spotify.get_authorize_url(false)?;
        prepare_cache(&spotify.config.cache_path, username)?;
        let cached = block_on(async {
            if let Ok(Some(token)) = spotify.read_token_cache(true).await {
                let expired = token.is_expired();
                *spotify.get_token().lock().await.unwrap() = Some(token);
                if !expired {
                    return Ok(true);
                }
                if let Ok(Some(token)) = spotify.refetch_token().await {
                    *spotify.get_token().lock().await.unwrap() = Some(token);
                    return Ok(true);
                }
            }
            Ok(false)
        })?;
        if !cached {
            // Bind before opening the browser. The async callback can be cancelled
            // by the same deadline as API requests; no blocking stdin/listener.
            let listener = std::net::TcpListener::bind("127.0.0.1:8888")?;
            listener.set_nonblocking(true)?;
            crate::adapters::platform::system::openfile::openfile(&url);
            block_on(async {
                let listener = tokio::net::TcpListener::from_std(listener)?;
                let code = callback_code(&spotify, &listener).await?;
                spotify.request_token(&code).await
            })?;
        }
        block_on(spotify.write_token_cache())?;
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

fn prepare_cache(path: &std::path::Path, username: &str) -> std::io::Result<()> {
    if !path.exists()
        && !username.is_empty()
        && username
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        let old = std::path::PathBuf::from(format!(".cache-{username}"));
        if old.is_file() && !old.is_symlink() {
            crate::adapters::config::atomic_replace(path, &std::fs::read(old)?)
                .map_err(|_| std::io::Error::other("Cannot migrate Spotify cache"))?;
        }
    }
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
        options.mode(0o600).custom_flags(libc::O_NOFOLLOW);
        let file = options.open(path)?;
        file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
    }
    #[cfg(not(unix))]
    {
        options.open(path)?;
    }
    Ok(())
}

async fn callback_code(
    spotify: &AuthCodeSpotify,
    listener: &tokio::net::TcpListener,
) -> Result<String, rspotify::ClientError> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let (mut stream, _) = listener.accept().await?;
    let mut request = Vec::new();
    while !request.windows(4).any(|w| w == b"\r\n\r\n") {
        let mut chunk = [0; 1024];
        let size = stream.read(&mut chunk).await?;
        if size == 0 || request.len() + size > 8192 {
            return Err(rspotify::ClientError::Cli("Invalid OAuth callback".into()));
        }
        request.extend_from_slice(&chunk[..size]);
    }
    let request = String::from_utf8_lossy(&request);
    let target = request
        .lines()
        .next()
        .unwrap_or("")
        .strip_prefix("GET ")
        .and_then(|line| line.strip_suffix(" HTTP/1.1"))
        .filter(|target| target.starts_with("/callback?"));
    let code = target
        .and_then(|target| spotify.parse_response_code(&format!("http://localhost:8888{target}")));
    let (status, body) = if code.is_some() {
        ("200 OK", "Spotify connected. You may close this window.")
    } else {
        ("400 Bad Request", "Invalid OAuth callback.")
    };
    stream.write_all(format!("HTTP/1.1 {status}\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes()).await?;
    code.ok_or_else(|| rspotify::ClientError::Cli("Invalid OAuth callback".into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn oauth_callback_validates_state_and_is_cancellable_without_input() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let spotify = AuthCodeSpotify::new(Credentials::new("test", "test"), OAuth::default());
        for valid in [true, false] {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let address = listener.local_addr().unwrap();
            let state = if valid {
                spotify.oauth.state.clone()
            } else {
                "wrong-state".into()
            };
            let client = async {
                let mut stream = tokio::net::TcpStream::connect(address).await.unwrap();
                stream.write_all(format!("GET /callback?code=test-code&state={state} HTTP/1.1\r\nHost: localhost\r\n\r\n").as_bytes()).await.unwrap();
                let mut response = String::new();
                stream.read_to_string(&mut response).await.unwrap();
                assert!(response.contains(if valid { "200 OK" } else { "400 Bad Request" }));
            };
            let (result, _) = tokio::join!(callback_code(&spotify, &listener), client);
            assert_eq!(result.is_ok(), valid);
            if valid {
                assert_eq!(result.unwrap(), "test-code");
            }
        }
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        assert!(tokio::time::timeout(
            std::time::Duration::from_millis(10),
            callback_code(&spotify, &listener)
        )
        .await
        .is_err());
    }
}
