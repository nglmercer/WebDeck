mod realtime;
use crate::{
    contracts::*,
    domain::{self, Error, Result},
    executor::Executor,
    sessions::Sessions,
    storage::{Assets, ConfigStore},
};
use axum::{
    extract::{ConnectInfo, DefaultBodyLimit, Multipart, Path, Request, State},
    http::{HeaderMap, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{delete, get, post},
    Json, Router,
};
use serde_json::{json, Value};
use std::{
    net::{IpAddr, SocketAddr},
    sync::Arc,
};
use tower_http::services::ServeDir;
#[derive(Clone)]
pub struct App {
    pub port: u16,
    pub plugins: Arc<Vec<PluginManifest>>,
    pub config: Arc<ConfigStore>,
    pub sessions: Arc<Sessions>,
    pub executor: Arc<Executor>,
    pub assets: Assets,
}
#[derive(Clone)]
struct Identity {
    local: bool,
    capabilities: Vec<Capability>,
}
impl Identity {
    fn require(&self, c: Capability) -> Result<()> {
        if self.capabilities.contains(&c) {
            Ok(())
        } else {
            Err(Error::new(ErrorCode::Forbidden, "Capability denied"))
        }
    }
    fn local(&self) -> Result<()> {
        if self.local {
            Ok(())
        } else {
            Err(Error::new(
                ErrorCode::Forbidden,
                "This operation requires a local administrator",
            ))
        }
    }
}
impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let status = match self.code {
            ErrorCode::Unauthorized => StatusCode::UNAUTHORIZED,
            ErrorCode::Forbidden => StatusCode::FORBIDDEN,
            ErrorCode::Conflict => StatusCode::CONFLICT,
            ErrorCode::CapacityExhausted => StatusCode::TOO_MANY_REQUESTS,
            ErrorCode::ShuttingDown => StatusCode::SERVICE_UNAVAILABLE,
            ErrorCode::ExecutionFailed | ErrorCode::PersistenceFailed => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
            _ => StatusCode::BAD_REQUEST,
        };
        (status, Json(self)).into_response()
    }
}
fn token(h: &HeaderMap) -> Result<Option<String>> {
    match h.get("authorization") {
        None => Ok(None),
        Some(v) => {
            let s = v
                .to_str()
                .map_err(|_| Error::new(ErrorCode::Unauthorized, "Invalid credential"))?;
            let t = s
                .strip_prefix("Bearer ")
                .filter(|t| !t.is_empty())
                .ok_or_else(|| Error::new(ErrorCode::Unauthorized, "Invalid credential"))?;
            Ok(Some(t.to_string()))
        }
    }
}
fn authority(h: &HeaderMap, local: bool, port: u16) -> Result<()> {
    let host = h
        .get("host")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| Error::new(ErrorCode::Forbidden, "Invalid host"))?;
    let parsed = reqwest::Url::parse(&format!("http://{host}")).map_err(|_| Error::invalid())?;
    let host_name = parsed.host_str().unwrap_or("").trim_matches(['[', ']']);
    let valid = host_name
        .parse::<IpAddr>()
        .is_ok_and(|a| !local || a.is_loopback())
        || (host_name == "localhost" && local);
    if parsed.port_or_known_default() != Some(port)
        || parsed.query().is_some()
        || parsed.fragment().is_some()
    {
        return Err(Error::new(ErrorCode::Forbidden, "Invalid server authority"));
    }
    if !valid
        || !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.path() != "/"
    {
        return Err(Error::new(
            ErrorCode::Forbidden,
            "Host is not an approved local address",
        ));
    }
    if let Some(origin) = h.get("origin") {
        let o = origin.to_str().map_err(|_| Error::invalid())?;
        let o = reqwest::Url::parse(o).map_err(|_| Error::invalid())?;
        if !matches!(o.scheme(), "http" | "https")
            || o.host_str() != parsed.host_str()
            || o.port_or_known_default() != parsed.port_or_known_default()
        {
            return Err(Error::new(
                ErrorCode::Forbidden,
                "Cross-origin request denied",
            ));
        }
    }
    Ok(())
}
fn network_allowed(ip: IpAddr, networks: &[String]) -> bool {
    if ip.is_loopback() || networks.is_empty() {
        return true;
    }
    networks.iter().any(|n| {
        let (a, p) = n
            .split_once('/')
            .map_or((n.as_str(), None), |(a, p)| (a, Some(p)));
        let Ok(a) = a.parse::<IpAddr>() else {
            return false;
        };
        match (ip, a) {
            (IpAddr::V4(ip), IpAddr::V4(a)) => {
                let p = p.and_then(|p| p.parse::<u32>().ok()).unwrap_or(32);
                let m = u32::MAX.checked_shl(32 - p).unwrap_or(0);
                u32::from(ip) & m == u32::from(a) & m
            }
            (IpAddr::V6(ip), IpAddr::V6(a)) => {
                let p = p.and_then(|p| p.parse::<u32>().ok()).unwrap_or(128);
                let m = u128::MAX.checked_shl(128 - p).unwrap_or(0);
                u128::from(ip) & m == u128::from(a) & m
            }
            _ => false,
        }
    })
}
async fn guard(State(a): State<App>, mut r: Request, next: Next) -> Response {
    let result = (|| {
        let peer = r
            .extensions()
            .get::<ConnectInfo<SocketAddr>>()
            .ok_or_else(|| Error::new(ErrorCode::Forbidden, "Peer identity unavailable"))?
            .0;
        let local = peer.ip().is_loopback();
        authority(r.headers(), local, a.port)?;
        if !network_allowed(
            peer.ip(),
            &a.config.last_valid().config.settings.allowed_networks,
        ) {
            return Err(Error::new(ErrorCode::Forbidden, "Network denied"));
        }
        if r.uri().path().starts_with("/socket.io") && r.headers().contains_key("authorization") {
            let t = token(r.headers())?;
            a.sessions.authorize(t.as_deref(), local)?;
        }
        if r.uri().path().starts_with("/api/v2/") {
            let t = token(r.headers())?;
            let capabilities = a.sessions.authorize(t.as_deref(), local)?;
            r.extensions_mut().insert(Identity {
                local,
                capabilities,
            });
        }
        Ok(())
    })();
    if let Err(e) = result {
        return e.into_response();
    }
    let mut response = next.run(r).await;
    let h = response.headers_mut();
    h.insert("x-content-type-options", "nosniff".parse().unwrap());
    h.insert("referrer-policy", "no-referrer".parse().unwrap());
    h.insert("cache-control", "no-store".parse().unwrap());
    response
}
pub fn router(a: App) -> Router {
    let layer = realtime::layer(a.clone());
    Router::new()
        .route("/", get(home))
        .route("/api/v2/boot", get(boot))
        .route("/api/v2/settings/boot", get(config))
        .route("/api/v2/config", get(config).put(replace))
        .route("/api/v2/settings", axum::routing::put(settings))
        .route("/api/v2/folders", post(folder))
        .route("/api/v2/folders/{id}", delete(remove_folder))
        .route("/api/v2/folders/{id}/buttons", post(button))
        .route(
            "/api/v2/folders/{folder}/buttons/{button}",
            axum::routing::put(update_button).delete(remove_button),
        )
        .route(
            "/api/v2/commands",
            get(catalog)
                .post(command)
                .layer(DefaultBodyLimit::max(65536)),
        )
        .route("/api/v2/usage", get(usage))
        .route("/api/v2/audio/devices", get(audio_devices))
        .route("/api/v2/translations", get(translations))
        .route("/api/v2/devices", get(devices).post(approve))
        .route("/api/v2/devices/{id}", delete(revoke))
        .route("/api/v2/assets", post(upload))
        .route("/api/v2/assets/{id}", get(asset))
        .route("/api/v2/native/selection", post(selection))
        .route("/api/v2/spotify/connect", post(spotify_connect))
        .route("/api/v2/spotify/callback", get(spotify_callback))
        .nest_service("/assets", ServeDir::new("frontend/dist/assets"))
        .nest_service("/static", ServeDir::new("static"))
        .layer(DefaultBodyLimit::max(16 * 1024 * 1024))
        .layer(layer)
        .layer(middleware::from_fn_with_state(a.clone(), guard))
        .with_state(a)
}
async fn home() -> Response {
    match tokio::fs::read("frontend/dist/index.html").await {
        Ok(b) => ([("content-type", "text/html; charset=utf-8")], b).into_response(),
        Err(_) => (
            StatusCode::SERVICE_UNAVAILABLE,
            "Build the frontend before starting WebDeck",
        )
            .into_response(),
    }
}
async fn boot(
    State(a): State<App>,
    axum::Extension(i): axum::Extension<Identity>,
) -> Result<Json<DeckBoot>> {
    i.require(Capability::Read)?;
    let s = a.config.snapshot()?;
    let mut layout = s.config.layout;
    layout.extensions.clear();
    let mut button_capabilities = std::collections::BTreeMap::new();
    for f in &mut layout.folders {
        f.extensions.clear();
        for b in &mut f.buttons {
            b.extensions.clear();
            if let ButtonAction::Command { command } = &mut b.action {
                let r = resolve_request(
                    &a,
                    CommandRequest {
                        request_id: "boot".into(),
                        command: command.clone(),
                    },
                )?;
                button_capabilities.insert(b.id.clone(), r.command.capability());
                *command = Command::Button {
                    button_id: b.id.clone(),
                };
            }
        }
    }
    Ok(Json(DeckBoot {
        api_version: 2,
        revision: s.revision,
        layout,
        language: s.config.settings.language,
        can_edit: i.capabilities.contains(&Capability::Settings),
        capabilities: i.capabilities,
        button_capabilities,
    }))
}
fn resolve_request(a: &App, mut r: CommandRequest) -> Result<CommandRequest> {
    for _ in 0..=8 {
        let Command::Button { button_id } = &r.command else {
            return Ok(r);
        };
        let c = a.config.snapshot()?.config;
        r.command = c
            .layout
            .folders
            .iter()
            .flat_map(|f| &f.buttons)
            .find(|b| &b.id == button_id)
            .and_then(|b| {
                if let ButtonAction::Command { command } = &b.action {
                    Some(command.clone())
                } else {
                    None
                }
            })
            .ok_or_else(Error::invalid)?;
    }
    Err(Error::new(
        ErrorCode::InvalidInput,
        "Button references exceed nesting limit",
    ))
}

async fn config(
    State(a): State<App>,
    axum::Extension(i): axum::Extension<Identity>,
) -> Result<Json<ConfigResponse>> {
    i.require(Capability::Settings)?;
    Ok(Json(a.config.snapshot()?))
}
async fn replace(
    State(a): State<App>,
    axum::Extension(i): axum::Extension<Identity>,
    Json(r): Json<ConfigRequest>,
) -> Result<Json<ConfigResponse>> {
    i.require(Capability::Settings)?;
    Ok(Json(a.config.mutate(r.revision, |c| {
        *c = r.config;
        Ok(())
    })?))
}
async fn settings(
    State(a): State<App>,
    axum::Extension(i): axum::Extension<Identity>,
    Json(r): Json<SettingsRequest>,
) -> Result<Json<ConfigResponse>> {
    i.require(Capability::Settings)?;
    Ok(Json(a.config.mutate(r.revision, |c| {
        c.settings = r.settings;
        Ok(())
    })?))
}
async fn folder(
    State(a): State<App>,
    axum::Extension(i): axum::Extension<Identity>,
    Json(r): Json<FolderRequest>,
) -> Result<Json<ConfigResponse>> {
    i.require(Capability::Settings)?;
    Ok(Json(a.config.mutate(r.revision, |c| {
        c.layout.folders.push(r.folder);
        Ok(())
    })?))
}
async fn remove_folder(
    State(a): State<App>,
    axum::Extension(i): axum::Extension<Identity>,
    Path(id): Path<String>,
    Json(r): Json<RevisionRequest>,
) -> Result<Json<ConfigResponse>> {
    i.require(Capability::Settings)?;
    Ok(Json(a.config.mutate(r.revision, |c| {
        if !c.layout.folders.iter().any(|f| f.id == id) {
            return Err(Error::invalid());
        }
        c.layout.folders.retain(|f| f.id != id);
        Ok(())
    })?))
}
async fn button(
    State(a): State<App>,
    axum::Extension(i): axum::Extension<Identity>,
    Path(id): Path<String>,
    Json(r): Json<ButtonRequest>,
) -> Result<Json<ConfigResponse>> {
    i.require(Capability::Settings)?;
    Ok(Json(a.config.mutate(r.revision, |c| {
        c.layout
            .folders
            .iter_mut()
            .find(|f| f.id == id)
            .ok_or_else(Error::invalid)?
            .buttons
            .push(r.button);
        Ok(())
    })?))
}
async fn update_button(
    State(a): State<App>,
    axum::Extension(i): axum::Extension<Identity>,
    Path((folder, id)): Path<(String, String)>,
    Json(r): Json<ButtonRequest>,
) -> Result<Json<ConfigResponse>> {
    i.require(Capability::Settings)?;
    if r.button.id != id {
        return Err(Error::invalid());
    }
    Ok(Json(a.config.mutate(r.revision, |c| {
        let f = c
            .layout
            .folders
            .iter_mut()
            .find(|f| f.id == folder)
            .ok_or_else(Error::invalid)?;
        *f.buttons
            .iter_mut()
            .find(|b| b.id == id)
            .ok_or_else(Error::invalid)? = r.button;
        Ok(())
    })?))
}
async fn remove_button(
    State(a): State<App>,
    axum::Extension(i): axum::Extension<Identity>,
    Path((folder, id)): Path<(String, String)>,
    Json(r): Json<RevisionRequest>,
) -> Result<Json<ConfigResponse>> {
    i.require(Capability::Settings)?;
    Ok(Json(a.config.mutate(r.revision, |c| {
        let f = c
            .layout
            .folders
            .iter_mut()
            .find(|f| f.id == folder)
            .ok_or_else(Error::invalid)?;
        if !f.buttons.iter().any(|b| b.id == id) {
            return Err(Error::invalid());
        }
        f.buttons.retain(|b| b.id != id);
        Ok(())
    })?))
}
async fn catalog(
    State(a): State<App>,
    axum::Extension(i): axum::Extension<Identity>,
) -> Result<Json<Value>> {
    i.require(Capability::Read)?;
    let catalog: Vec<Value> =
        serde_json::from_str(include_str!("../contracts/catalog.json")).expect("generated catalog");
    Ok(Json(
        json!({"api_version":2,"plugins":a.plugins.iter().filter(|p|p.actions.iter().any(|a|a.capabilities.iter().all(|c|i.capabilities.contains(c)))).collect::<Vec<_>>(),"commands":catalog.into_iter().filter(|c|serde_json::from_value::<Capability>(c["capability"].clone()).is_ok_and(|c|i.capabilities.contains(&c))).collect::<Vec<_>>()}),
    ))
}
fn command_event(id: String, result: Result<Value>) -> Value {
    match result {
        Ok(result) => json!({"api_version":2,"request_id":id,"state":"completed","result":result}),
        Err(e) => {
            json!({"api_version":2,"request_id":id,"state":"failed","code":e.code,"message":e.message})
        }
    }
}
async fn command(
    State(a): State<App>,
    axum::Extension(i): axum::Extension<Identity>,
    Json(r): Json<CommandRequest>,
) -> Response {
    let id = r.request_id.clone();
    let result = match resolve_request(&a, r) {
        Ok(r) => a.executor.execute(r, i.capabilities, || {}).await,
        Err(e) => Err(e),
    };
    let status = match &result {
        Ok(_) => StatusCode::OK,
        Err(e) => match e.code {
            ErrorCode::Forbidden => StatusCode::FORBIDDEN,
            ErrorCode::CapacityExhausted => StatusCode::TOO_MANY_REQUESTS,
            ErrorCode::ShuttingDown => StatusCode::SERVICE_UNAVAILABLE,
            ErrorCode::InvalidInput => StatusCode::BAD_REQUEST,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        },
    };
    (status, Json(command_event(id, result))).into_response()
}

async fn usage(axum::Extension(i): axum::Extension<Identity>) -> Result<Json<Value>> {
    i.require(Capability::Read)?;
    let v = tokio::task::spawn_blocking(crate::native::usage)
        .await
        .map_err(|_| Error::execution())?;
    Ok(Json(json!({"api_version":2,"usage":v})))
}
async fn devices(
    State(a): State<App>,
    axum::Extension(i): axum::Extension<Identity>,
) -> Result<Json<DeviceList>> {
    i.local()?;
    i.require(Capability::Settings)?;
    Ok(Json(DeviceList {
        api_version: 2,
        devices: a.sessions.list(),
    }))
}
async fn approve(
    State(a): State<App>,
    axum::Extension(i): axum::Extension<Identity>,
    Json(r): Json<DeviceRequest>,
) -> Result<Json<DeviceApproval>> {
    i.local()?;
    i.require(Capability::Settings)?;
    Ok(Json(a.sessions.approve(r)?))
}
async fn revoke(
    State(a): State<App>,
    axum::Extension(i): axum::Extension<Identity>,
    Path(id): Path<String>,
) -> Result<Json<Value>> {
    i.local()?;
    i.require(Capability::Settings)?;
    a.sessions.revoke(&id)?;
    Ok(Json(json!({"api_version":2,"revoked":true})))
}
async fn upload(
    State(a): State<App>,
    axum::Extension(i): axum::Extension<Identity>,
    mut m: Multipart,
) -> Result<Json<FileSource>> {
    i.require(Capability::Settings)?;
    let f = m
        .next_field()
        .await
        .map_err(|_| Error::invalid())?
        .ok_or_else(Error::invalid)?;
    let extension = f
        .file_name()
        .and_then(|n| n.rsplit('.').next())
        .unwrap_or("")
        .to_lowercase();
    let bytes = f.bytes().await.map_err(|_| Error::invalid())?;
    Ok(Json(a.assets.upload(&extension, &bytes)?))
}
async fn asset(
    State(a): State<App>,
    axum::Extension(i): axum::Extension<Identity>,
    Path(id): Path<String>,
) -> Result<Response> {
    i.require(Capability::Read)?;
    let b = a.assets.read(&id)?;
    let mime = match id.rsplit('.').next().unwrap_or("") {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "ogg" => "audio/ogg",
        "css" => "text/css; charset=utf-8",
        "svg" => "image/svg+xml",
        _ => "application/octet-stream",
    };
    Ok((
        [
            ("content-type", mime),
            ("content-security-policy", "sandbox; default-src 'none'"),
        ],
        b,
    )
        .into_response())
}
async fn selection(
    axum::Extension(i): axum::Extension<Identity>,
    Json(r): Json<NativeSelection>,
) -> Result<Json<Value>> {
    i.local()?;
    i.require(Capability::Settings)?;
    domain::validate(
        "NativeSelection",
        &serde_json::to_value(&r).map_err(|_| Error::invalid())?,
    )?;
    let p = if r.kind == "folder" {
        rfd::AsyncFileDialog::new().pick_folder().await
    } else {
        rfd::AsyncFileDialog::new().pick_file().await
    };
    Ok(Json(
        json!({"api_version":2,"source":p.map(|p|FileSource::External{path:p.path().to_string_lossy().into_owned()})}),
    ))
}
fn oauth_client(a: &App) -> rspotify::AuthCodeSpotify {
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
fn oauth_states() -> &'static std::sync::Mutex<std::collections::HashMap<String, std::time::Instant>>
{
    static S: std::sync::OnceLock<
        std::sync::Mutex<std::collections::HashMap<String, std::time::Instant>>,
    > = std::sync::OnceLock::new();
    S.get_or_init(Default::default)
}
async fn spotify_connect(
    State(a): State<App>,
    axum::Extension(i): axum::Extension<Identity>,
) -> Result<Json<Value>> {
    i.local()?;
    i.require(Capability::Settings)?;
    let c = oauth_client(&a);
    let state = domain::id();
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
struct OAuthCallback {
    code: String,
    state: String,
}
async fn spotify_callback(
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
    c.request_token(&r.code)
        .await
        .map_err(|_| Error::execution())?;
    let token = c.token.lock().await.map_err(|_| Error::execution())?;
    crate::storage::atomic_replace(
        &a.assets.root.join("spotify-token.json"),
        &serde_json::to_vec(&*token).map_err(|_| Error::execution())?,
    )?;
    Ok(axum::response::Redirect::to("/").into_response())
}

async fn translations(
    State(a): State<App>,
    axum::Extension(i): axum::Extension<Identity>,
) -> Result<Json<Value>> {
    i.require(Capability::Read)?;
    let language = a.config.last_valid().config.settings.language;
    let root = std::path::Path::new("webdeck/translations");
    let mut result = std::collections::BTreeMap::new();
    for name in ["en_US", language.as_str()] {
        if let Ok(text) = std::fs::read_to_string(root.join(format!("{name}.lang"))) {
            for line in text.lines() {
                if line.starts_with('#') || line.starts_with("//") {
                    continue;
                }
                if let Some((k, v)) = line.split_once('=') {
                    result.insert(k.trim().to_string(), v.trim().to_string());
                }
            }
        }
    }
    let languages = std::fs::read_dir(root)
        .map_err(|_| Error::execution())?
        .flatten()
        .filter_map(|e| {
            e.path()
                .file_stem()
                .and_then(|s| s.to_str())
                .map(str::to_owned)
        })
        .collect::<Vec<_>>();
    Ok(Json(
        json!({"api_version":2,"translations":result,"languages":languages}),
    ))
}

async fn audio_devices(
    axum::Extension(i): axum::Extension<Identity>,
) -> Result<Json<AudioDevices>> {
    i.require(Capability::Settings)?;
    let d = tokio::task::spawn_blocking(|| {
        use rodio::cpal::traits::{DeviceTrait, HostTrait};
        let h = rodio::cpal::default_host();
        let inputs = h
            .input_devices()
            .map_err(|_| Error::execution())?
            .filter_map(|d| d.description().ok().map(|n| n.name().to_string()))
            .collect();
        let outputs = h
            .output_devices()
            .map_err(|_| Error::execution())?
            .filter_map(|d| d.description().ok().map(|n| n.name().to_string()))
            .collect();
        Ok::<_, Error>(AudioDevices {
            api_version: 2,
            inputs,
            outputs,
        })
    })
    .await
    .map_err(|_| Error::execution())??;
    Ok(Json(d))
}
