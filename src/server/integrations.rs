use super::*;

pub struct IntegrationHealthCache {
    entries: std::collections::HashMap<String, CachedHealth>,
    generation: u64,
    checks: Arc<tokio::sync::Semaphore>,
}
impl Default for IntegrationHealthCache {
    fn default() -> Self {
        Self {
            entries: Default::default(),
            generation: 0,
            checks: Arc::new(tokio::sync::Semaphore::new(2)),
        }
    }
}
struct CachedHealth {
    definition: IntegrationDefinition,
    fingerprint: String,
    health: IntegrationHealth,
}
fn fingerprint(definition: &IntegrationDefinition, settings: &Value) -> String {
    use sha2::{Digest, Sha256};
    let selected = definition
        .configuration
        .iter()
        .chain(&definition.required_settings)
        .map(|pointer| {
            (
                pointer,
                settings.pointer(pointer).cloned().unwrap_or(Value::Null),
            )
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&(definition, selected)).expect("serializable metadata"))
    )
}
impl IntegrationHealthCache {
    pub(super) fn clear(&mut self) {
        self.entries.clear();
        self.generation = self.generation.wrapping_add(1);
    }
    pub(super) fn invalidate_changed(&mut self, settings: &Settings) {
        let settings = serde_json::to_value(settings).expect("serializable settings");
        self.entries
            .retain(|_, cached| cached.fingerprint == fingerprint(&cached.definition, &settings));
        // In-flight probes cannot publish a result across a settings transaction.
        self.generation = self.generation.wrapping_add(1);
    }
}
fn now_seconds() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

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

async fn definitions(a: &App) -> Result<AutomationMetadata> {
    let owner = a.clone();
    let snapshot = blocking(a.queries.clone(), move || owner.executor.management(None))
        .await
        .ok();
    let plugins = snapshot
        .as_ref()
        .and_then(|v| serde_json::from_value::<Vec<PluginManifest>>(v["plugins"].clone()).ok())
        .unwrap_or_else(|| a.plugins.as_ref().clone());
    let disabled = snapshot
        .as_ref()
        .and_then(|v| serde_json::from_value::<Vec<String>>(v["disabled_plugins"].clone()).ok())
        .unwrap_or_default();
    crate::automation::collect(&plugins, &disabled)
}

async fn status_snapshot(a: &App, metadata: &AutomationMetadata) -> Result<IntegrationStatus> {
    let settings =
        serde_json::to_value(a.config.last_valid().config.settings).expect("serializable settings");
    let mut integrations = std::collections::BTreeMap::new();
    for definition in &metadata.integrations {
        let configured = definition.required_settings.iter().all(|p| {
            settings
                .pointer(p)
                .is_some_and(|v| !v.is_null() && v.as_str().is_none_or(|s| !s.trim().is_empty()))
        });
        let state = if !configured {
            Some(IntegrationState::NotConfigured)
        } else if let Some(asset) = &definition.authorization_asset {
            let root = a.assets.root.clone();
            let asset = asset.clone();
            let saved = blocking(a.io.clone(), move || {
                Ok(crate::runtime::plugins::confined(&root, &asset).is_ok())
            })
            .await?;
            if !saved {
                Some(IntegrationState::AuthorizationRequired)
            } else if definition.probe.is_none() {
                Some(IntegrationState::AuthorizationSaved)
            } else {
                None
            }
        } else {
            None
        };
        let key = fingerprint(definition, &settings);
        let mut cache = a
            .integration_health
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        if cache
            .entries
            .get(&definition.id)
            .is_some_and(|entry| entry.fingerprint != key)
        {
            cache.entries.remove(&definition.id);
        }
        let health = if let Some(state) = state {
            cache.entries.remove(&definition.id);
            IntegrationHealth {
                state,
                checked_at: 0,
            }
        } else {
            cache
                .entries
                .get(&definition.id)
                .map(|entry| entry.health.clone())
                .unwrap_or(IntegrationHealth {
                    state: IntegrationState::NotTested,
                    checked_at: 0,
                })
        };
        integrations.insert(definition.id.clone(), health);
    }
    // Compatibility projection for existing v2 clients. All evaluation uses the registry above.
    Ok(IntegrationStatus {
        api_version: 2,
        obs: integrations
            .get("obs")
            .map(|h| h.state)
            .unwrap_or(IntegrationState::NotConfigured),
        spotify: integrations
            .get("spotify")
            .map(|h| h.state)
            .unwrap_or(IntegrationState::NotConfigured),
        checked_at: integrations
            .values()
            .map(|h| h.checked_at)
            .max()
            .unwrap_or(0),
        integrations: Some(integrations),
    })
}
pub(super) async fn integration_status(
    State(a): State<App>,
    axum::Extension(i): axum::Extension<Identity>,
) -> Result<Json<IntegrationStatus>> {
    i.local()?;
    i.require(Capability::Settings)?;
    let metadata = definitions(&a).await?;
    Ok(Json(status_snapshot(&a, &metadata).await?))
}
pub(super) async fn check_integration(
    State(a): State<App>,
    axum::Extension(i): axum::Extension<Identity>,
    Path(id): Path<String>,
) -> Result<Json<IntegrationStatus>> {
    i.local()?;
    i.require(Capability::Settings)?;
    let checks = a
        .integration_health
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .checks
        .clone();
    let _permit = checks.try_acquire_owned().map_err(|_| {
        Error::new(
            ErrorCode::CapacityExhausted,
            "A connection check is already running. Try again shortly.",
        )
    })?;
    let metadata = definitions(&a).await?;
    let definition = metadata
        .integrations
        .iter()
        .find(|d| d.id == id)
        .ok_or_else(|| Error::new(ErrorCode::InvalidInput, "Unknown integration"))?;
    let status = status_snapshot(&a, &metadata).await?;
    if status
        .integrations
        .as_ref()
        .and_then(|items| items.get(&id))
        .is_some_and(|h| {
            matches!(
                h.state,
                IntegrationState::NotConfigured | IntegrationState::AuthorizationRequired
            )
        })
    {
        return Ok(Json(status));
    }
    let Some(probe) = &definition.probe else {
        return Ok(Json(status));
    };
    i.require(probe.capability())?;
    let settings =
        serde_json::to_value(a.config.last_valid().config.settings).expect("serializable settings");
    let key = fingerprint(definition, &settings);
    let generation = a
        .integration_health
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .generation;
    let result = a
        .executor
        .execute(
            CommandRequest {
                request_id: domain::id()?,
                command: probe.clone(),
            },
            i.capabilities,
            || {},
        )
        .await;
    if let Err(error) = &result {
        if matches!(error.code, ErrorCode::Forbidden | ErrorCode::Unauthorized) {
            return Err(error.clone());
        }
    }
    let connected = result.is_ok_and(|result| {
        definition.success.as_ref().is_none_or(|predicate| {
            result
                .pointer(&predicate.pointer)
                .is_some_and(|value| match &predicate.equals {
                    Some(expected) => value == expected,
                    None => !value.is_null() && value.as_str().is_none_or(|s| !s.is_empty()),
                })
        })
    });
    let health = IntegrationHealth {
        state: if connected {
            IntegrationState::Connected
        } else {
            IntegrationState::Failed
        },
        checked_at: now_seconds(),
    };
    let current_settings =
        serde_json::to_value(a.config.last_valid().config.settings).expect("serializable settings");
    {
        let mut cache = a
            .integration_health
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        if cache.generation == generation && fingerprint(definition, &current_settings) == key {
            cache.entries.insert(
                id,
                CachedHealth {
                    definition: definition.clone(),
                    fingerprint: key,
                    health,
                },
            );
        }
    }
    Ok(Json(status_snapshot(&a, &metadata).await?))
}
