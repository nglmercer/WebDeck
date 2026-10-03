use super::*;

const SPOTIFY_SCOPES:&str="user-read-playback-state user-modify-playback-state user-library-modify playlist-modify-public playlist-modify-private user-follow-modify user-follow-read user-library-read";
pub(super) fn oauth_states(
) -> &'static std::sync::Mutex<std::collections::HashMap<String, std::time::Instant>> {
    static S: std::sync::OnceLock<
        std::sync::Mutex<std::collections::HashMap<String, std::time::Instant>>,
    > = std::sync::OnceLock::new();
    S.get_or_init(Default::default)
}
pub(super) async fn spotify_connect(
    State(a): State<App>,
    axum::Extension(i): axum::Extension<Identity>,
) -> Result<Json<Value>> {
    i.local()?;
    i.require(Capability::Settings)?;
    let settings = a.config.last_valid().config.settings.spotify;
    if settings.client_id.is_empty() || settings.client_secret.is_empty() {
        return Err(Error::new(
            ErrorCode::InvalidInput,
            "Configure Spotify from local settings first",
        ));
    }
    let state = domain::id()?;
    let mut u = reqwest::Url::parse("https://accounts.spotify.com/authorize")
        .map_err(|_| Error::execution())?;
    u.query_pairs_mut()
        .append_pair("response_type", "code")
        .append_pair("client_id", &settings.client_id)
        .append_pair("redirect_uri", &settings.redirect_uri)
        .append_pair("scope", SPOTIFY_SCOPES)
        .append_pair("state", &state);
    let mut states = oauth_states().lock().unwrap_or_else(|p| p.into_inner());
    states.retain(|_, v| v.elapsed().as_secs() < 300);
    if states.len() >= 16 {
        return Err(Error::new(
            ErrorCode::CapacityExhausted,
            "Too many connection attempts",
        ));
    }
    states.insert(state, std::time::Instant::now());
    Ok(Json(json!({"api_version":2,"url":u.as_str()})))
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct OAuthCallback {
    code: String,
    state: String,
}
pub(super) async fn spotify_callback(
    State(a): State<App>,
    axum::Extension(i): axum::Extension<Identity>,
    axum::extract::Query(r): axum::extract::Query<OAuthCallback>,
) -> Result<Response> {
    i.local()?;
    i.require(Capability::Settings)?;
    let started = oauth_states()
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .remove(&r.state)
        .ok_or_else(Error::invalid)?;
    if started.elapsed().as_secs() > 300 {
        return Err(Error::invalid());
    }
    if r.code.len() > 4096 {
        return Err(Error::invalid());
    }
    let settings = a.config.last_valid().config.settings.spotify;
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|_| Error::execution())?;
    let body = reqwest::Url::parse_with_params(
        "https://accounts.spotify.com/api/token",
        [
            ("grant_type", "authorization_code"),
            ("code", r.code.as_str()),
            ("redirect_uri", settings.redirect_uri.as_str()),
        ],
    )
    .map_err(|_| Error::invalid())?
    .query()
    .unwrap_or("")
    .to_owned();
    let mut response = client
        .post("https://accounts.spotify.com/api/token")
        .basic_auth(settings.client_id, Some(settings.client_secret))
        .header("content-type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .map_err(|_| Error::execution())?;
    if !response.status().is_success() {
        return Err(Error::new(
            ErrorCode::ExecutionFailed,
            "Spotify authorization exchange failed",
        ));
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| Error::execution())? {
        if bytes.len() + chunk.len() > 65536 {
            return Err(Error::execution());
        }
        bytes.extend_from_slice(&chunk);
    }
    let mut token: Value = serde_json::from_slice(&bytes).map_err(|_| Error::execution())?;
    if !token["access_token"].is_string() {
        return Err(Error::execution());
    }
    let expires = token["expires_in"]
        .as_u64()
        .filter(|n| *n <= 31536000)
        .ok_or_else(Error::execution)?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    token["expires_at_ms"] = json!(now + expires * 1000);
    let bytes = serde_json::to_vec(&token).map_err(|_| Error::execution())?;
    let destination = a.assets.root.join("spotify-token.json");
    blocking(a.io.clone(), move || {
        crate::storage::atomic_replace(&destination, &bytes)
    })
    .await?;
    Ok(axum::response::Redirect::to("/").into_response())
}

async fn status_snapshot(a: &App) -> IntegrationStatus {
    let settings = a.config.last_valid().config.settings;
    let spotify = if settings.spotify.client_id.trim().is_empty()
        || settings.spotify.client_secret.trim().is_empty()
    {
        IntegrationState::NotConfigured
    } else if tokio::fs::metadata(a.assets.root.join("spotify-token.json"))
        .await
        .is_ok_and(|m| m.is_file())
    {
        IntegrationState::AuthorizationSaved
    } else {
        IntegrationState::AuthorizationRequired
    };
    IntegrationStatus {
        api_version: 2,
        obs: if settings.obs.host.trim().is_empty() {
            IntegrationState::NotConfigured
        } else {
            IntegrationState::NotTested
        },
        spotify,
        checked_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
    }
}
pub(super) async fn integration_status(
    State(a): State<App>,
    axum::Extension(i): axum::Extension<Identity>,
) -> Result<Json<IntegrationStatus>> {
    i.local()?;
    i.require(Capability::Settings)?;
    Ok(Json(status_snapshot(&a).await))
}
pub(super) async fn check_obs(
    State(a): State<App>,
    axum::Extension(i): axum::Extension<Identity>,
) -> Result<Json<IntegrationStatus>> {
    i.local()?;
    i.require(Capability::Settings)?;
    static CHECKS: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(2);
    let _permit = CHECKS.try_acquire().map_err(|_| {
        Error::new(
            ErrorCode::CapacityExhausted,
            "A connection check is already running. Try again shortly.",
        )
    })?;
    let mut status = status_snapshot(&a).await;
    if status.obs == IntegrationState::NotConfigured {
        return Ok(Json(status));
    }
    // Read-only connection health goes through the same runtime as OBS actions.
    let connected = a
        .executor
        .execute(
            CommandRequest {
                request_id: "obs-health".into(),
                command: Command::Obs {
                    action: "get_version".into(),
                    target: String::new(),
                },
            },
            vec![Capability::Network],
            || {},
        )
        .await
        .is_ok_and(|v| v["obsVersion"].as_str().is_some_and(|s| !s.is_empty()));
    status.obs = if connected {
        IntegrationState::Connected
    } else {
        IntegrationState::Failed
    };
    Ok(Json(status))
}
