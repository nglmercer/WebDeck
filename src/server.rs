mod auth;
use auth::*;
mod configuration;
use configuration::*;
mod devices;
use devices::*;
mod assets;
use assets::*;
mod integrations;
use integrations::*;
mod metadata;
use metadata::*;
mod commands;
use commands::*;
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
    pub io: Arc<tokio::sync::Semaphore>,
    pub queries: Arc<tokio::sync::Semaphore>,
    pub authorization: Arc<tokio::sync::Semaphore>,
}
// Permits belong to the blocking task, so observer cancellation cannot release capacity early.
async fn blocking<T: Send + 'static>(
    admission: Arc<tokio::sync::Semaphore>,
    work: impl FnOnce() -> Result<T> + Send + 'static,
) -> Result<T> {
    let permit = admission.try_acquire_owned().map_err(|_| {
        Error::new(
            ErrorCode::CapacityExhausted,
            "Blocking work capacity exhausted",
        )
    })?;
    tokio::task::spawn_blocking(move || {
        let _owned = permit;
        work()
    })
    .await
    .map_err(|_| Error::execution())?
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
pub fn router(a: App) -> Router {
    let layer = realtime::layer(a.clone());
    Router::new()
        .route("/", get(home))
        .route("/api/v2/boot", get(boot))
        .route("/api/v2/version", get(version))
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
        .route(
            "/api/v2/runtime/plugins/{id}",
            axum::routing::put(plugin_enabled),
        )
        .route("/api/v2/runtime", get(runtime_status))
        .route("/api/v2/runtime/reload", post(runtime_reload))
        .route("/api/v2/usage", get(usage))
        .route("/api/v2/audio/devices", get(audio_devices))
        .route("/api/v2/translations", get(translations))
        .route("/api/v2/devices", get(devices).post(approve))
        .route("/api/v2/devices/{id}", delete(revoke))
        .route("/api/v2/assets", post(upload))
        .route("/api/v2/assets/{id}", get(asset))
        .route("/api/v2/native/selection", post(selection))
        .route("/api/v2/integrations/status", get(integration_status))
        .route("/api/v2/integrations/obs/check", post(check_obs))
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
        Ok(b) if frontend_matches_contract(&b) => (
            [("content-type", "text/html; charset=utf-8"), ("cache-control", "no-store")],
            b,
        ).into_response(),
        Ok(_) => (
            StatusCode::SERVICE_UNAVAILABLE,
            [("cache-control", "no-store")],
            "Frontend build is out of date. Run npm ci --prefix frontend && npm run build --prefix frontend, then reload WebDeck. Your configuration is unchanged.",
        ).into_response(),
        Err(_) => (
            StatusCode::SERVICE_UNAVAILABLE,
            "Build the frontend before starting WebDeck",
        )
            .into_response(),
    }
}

fn frontend_matches_contract(html: &[u8]) -> bool {
    use sha2::{Digest, Sha256};
    let digest = format!(
        "{:x}",
        Sha256::digest(include_bytes!("../contracts/v2.schema.json"))
    );
    String::from_utf8_lossy(html)
        .contains(&format!("name=\"webdeck-contract\" content=\"{digest}\""))
}

#[cfg(test)]
mod blocking_tests {
    use super::*;

    #[test]
    fn frontend_build_must_match_the_compiled_contract() {
        use sha2::{Digest, Sha256};
        let digest = format!(
            "{:x}",
            Sha256::digest(include_bytes!("../contracts/v2.schema.json"))
        );
        assert!(frontend_matches_contract(
            format!("<meta name=\"webdeck-contract\" content=\"{digest}\">").as_bytes()
        ));
        assert!(!frontend_matches_contract(
            b"<meta name=\"webdeck-contract\" content=\"old\">"
        ));
        assert!(!frontend_matches_contract(b"<html>old bundle</html>"));
    }

    #[tokio::test]
    async fn cancelled_observer_does_not_release_blocking_capacity() {
        let admission = Arc::new(tokio::sync::Semaphore::new(1));
        let (started, ready) = tokio::sync::oneshot::channel();
        let (release, wait) = std::sync::mpsc::channel();
        let owner = admission.clone();
        let observer = tokio::spawn(async move {
            blocking(owner, move || {
                let _ = started.send(());
                let _ = wait.recv();
                Ok(())
            })
            .await
        });
        ready.await.unwrap();
        observer.abort();
        assert!(observer.await.unwrap_err().is_cancelled());
        assert_eq!(admission.available_permits(), 0);
        assert_eq!(
            blocking(admission.clone(), || Ok(()))
                .await
                .unwrap_err()
                .code,
            ErrorCode::CapacityExhausted
        );
        release.send(()).unwrap();
        let permit = tokio::time::timeout(std::time::Duration::from_secs(2), admission.acquire())
            .await
            .unwrap()
            .unwrap();
        drop(permit);
        assert_eq!(admission.available_permits(), 1);
    }
}
