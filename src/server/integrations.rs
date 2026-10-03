use super::*;

pub(super) fn oauth_client(a: &App) -> rspotify::AuthCodeSpotify {
    let s = a.config.last_valid().config.settings.spotify;
    rspotify::AuthCodeSpotify::new(
        rspotify::Credentials::new(&s.client_id, &s.client_secret),
        rspotify::OAuth {
            redirect_uri: s.redirect_uri,
            scopes: rspotify::scopes!(
                "user-read-playback-state",
                "user-modify-playback-state",
                "user-library-modify",
                "playlist-modify-public",
                "playlist-modify-private",
                "user-follow-modify",
                "user-follow-read",
                "user-library-read"
            ),
            ..Default::default()
        },
    )
}
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
    let c = oauth_client(&a);
    let state = domain::id()?;
    let url = c.get_authorize_url(false).map_err(|_| Error::execution())?;
    let mut u = reqwest::Url::parse(&url).map_err(|_| Error::execution())?;
    let query: Vec<(String, String)> = u
        .query_pairs()
        .filter(|(k, _)| k != "state")
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();
    u.query_pairs_mut()
        .clear()
        .extend_pairs(query)
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
    use rspotify::prelude::*;
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
    let c = oauth_client(&a);
    tokio::time::timeout(std::time::Duration::from_secs(10), c.request_token(&r.code))
        .await
        .map_err(|_| {
            Error::new(
                ErrorCode::ExecutionFailed,
                "Authorization exchange timed out",
            )
        })?
        .map_err(|_| Error::execution())?;
    let bytes = {
        let token = c.token.lock().await.map_err(|_| Error::execution())?;
        serde_json::to_vec(&*token).map_err(|_| Error::execution())?
    };
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
    let obs = a.config.last_valid().config.settings.obs;
    if status.obs == IntegrationState::NotConfigured {
        return Ok(Json(status));
    }
    // Check identification/version only; this never triggers recording, streaming, or a hotkey.
    let connected = tokio::time::timeout(std::time::Duration::from_secs(3), async {
        let mut client =
            obws::Client::connect(obs.host, obs.port as u16, Some(obs.password.as_str()))
                .await
                .ok()?;
        let result = client.general().version().await.is_ok();
        client.disconnect().await;
        Some(result)
    })
    .await
    .ok()
    .flatten()
    .unwrap_or(false);
    status.obs = if connected {
        IntegrationState::Connected
    } else {
        IntegrationState::Failed
    };
    Ok(Json(status))
}
