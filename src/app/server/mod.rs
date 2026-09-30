//! HTTP composition root. V2 commands, deck boot, configuration and devices
//! share application services and version-independent authorization.

mod assets;
mod middleware;
mod realtime;
mod routes_boot;
mod routes_config;
mod routes_upload;
mod security;
mod v2;

pub use assets::get_svgs;

use std::net::SocketAddr;
use std::sync::Arc;

use axum::{
    extract::DefaultBodyLimit,
    http::StatusCode,
    middleware as axum_middleware,
    response::{IntoResponse, Json, Response},
    routing::{get, post},
    Router,
};
use serde_json::json;
use tower_http::services::ServeDir;

use self::middleware::{after_request, check_local_network};
use self::realtime::{socketio_layer, usage};
use self::routes_boot::{boot, home, settings_boot};
use self::routes_config::{
    complete_save_config, create_folder, get_config_route, save_buttons_only, save_single_button,
    saveconfig,
};
use self::routes_upload::{get_config_file, upload_file, upload_filepath, upload_folderpath};
use crate::app::buttons::soundboard;
use crate::app::on_start::on_start;
use crate::app::tray::{change_server_state, ServerState};
use crate::app::utils::{
    args::get_args,
    firewall::{check_firewall_permission, fix_firewall_permission},
    global_variables::set_global_variable,
    logger::log,
    settings::get_config::get_port,
};

/// Server transport state; configuration transactions live in the application.
#[derive(Clone)]
pub struct AppState {
    pub local_ip: String,
    pub executor: Arc<crate::application::executor::CommandExecutor>,
}

/// Server startup/serve error (port of exceptions out of `run_server`).
#[derive(Debug)]
pub struct ServerError(pub String);

impl std::fmt::Display for ServerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Server error: {}", self.0)
    }
}

/// Port of the Flask `@app.errorhandler(Exception)` — logs like
/// `handle_exception` and returns the 500 JSON.
pub(crate) fn internal_error(
    context: &str,
    detail: String,
    req: Option<(&str, &str, &str)>,
) -> Response {
    log().exception(&detail, Some(context), true, true, true);
    if let Some((remote, method, url)) = req {
        log().httprequest(remote, method, url, 500);
    }
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(json!({"success": false, "message": detail})),
    )
        .into_response()
}

/// Route table shared by [`run_server`] and the routing regression tests.
/// (The SocketIO layer + TCP bind stay in [`run_server`].)
pub fn app_router(state: AppState) -> Router {
    Router::new()
        .route("/", get(home))
        .route("/api/v2/boot", get(boot))
        .route("/api/v2/settings/boot", get(settings_boot))
        .route("/usage", post(usage))
        .route("/save_config", post(saveconfig))
        .route("/COMPLETE_save_config", post(complete_save_config))
        .route("/save_single_button", post(save_single_button))
        .route("/save_buttons_only", post(save_buttons_only))
        // Both methods: boot + editor config loads POST, editor save flows GET.
        .route("/get_config", get(get_config_route).post(get_config_route))
        .route("/upload_folderpath", post(upload_folderpath))
        .route("/upload_filepath", post(upload_filepath))
        .route("/upload_file", post(upload_file))
        .route("/create_folder", post(create_folder))
        .route("/.config/{directory}/{filename}", get(get_config_file))
        .route(
            "/api/v2/commands",
            get(v2::catalog)
                .post(v2::command)
                .layer(DefaultBodyLimit::max(65536)),
        )
        .route("/api/v2/config", get(v2::get_config).post(v2::save_config))
        .route(
            "/api/v2/devices",
            get(v2::list_devices).post(v2::approve_device),
        )
        .route(
            "/api/v2/devices/{id}",
            axum::routing::delete(v2::revoke_device),
        )
        .nest_service("/static", ServeDir::new("static"))
        .nest_service("/assets", ServeDir::new("frontend/dist/assets"))
        .layer(axum_middleware::from_fn_with_state(
            state.clone(),
            check_local_network,
        ))
        .layer(axum_middleware::from_fn_with_state(
            state.clone(),
            security::guard,
        ))
        .layer(DefaultBodyLimit::max(16 * 1024 * 1024))
        .with_state(state)
        .layer(axum_middleware::from_fn(after_request))
}

/// Port of `run_server`.
///
/// Python runs `on_start()` + firewall checks at import time; Rust runs them
/// at the top of this function (same order, same conditions).
pub async fn run_server() -> Result<(), ServerError> {
    let (config, commands, local_ip) = on_start().await;
    crate::application::catalog::publish(commands);
    set_global_variable("config", config.clone());

    // Python starts the mic loop at import time when enabled.
    soundboard::mic::start_if_enabled();

    // Python module level: firewall bypass + local IP log.
    if config["settings"]["automatic_firewall_bypass"].as_bool() == Some(true)
        && !check_firewall_permission()
    {
        fix_firewall_permission();
    }
    log().info(&format!("Local IP address detected: {local_ip}"));

    change_server_state(ServerState::Running);

    let state = AppState {
        local_ip: local_ip.clone(),
        executor: crate::application::executor::shared(),
    };

    let app = app_router(state.clone());

    let host = get_args().host.clone().unwrap_or(local_ip);
    let port = get_port();
    let executor = state.executor.clone();
    let app = app
        .layer(socketio_layer(&host, port, state.clone()))
        .layer(axum_middleware::from_fn_with_state(
            state.clone(),
            check_local_network,
        ))
        .layer(axum_middleware::from_fn_with_state(state, security::guard));
    let listener = tokio::net::TcpListener::bind(format!("{host}:{port}"))
        .await
        .map_err(|e| ServerError(format!("Cannot bind {host}:{port}: {e}")))?;
    log().info(&format!("Serving on {host}:{port}"));
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(async move {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => crate::application::lifecycle::request_shutdown(),
            _ = crate::application::lifecycle::shutdown_requested() => (),
        }
        executor.drain().await;
        soundboard::mic::stop();
    })
    .await
    .map_err(|e| ServerError(format!("Server failed: {e}")))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    use crate::app::utils::settings::get_config::test_support::{config_guard, seed_config};

    fn test_state() -> AppState {
        AppState {
            local_ip: "127.0.0.1".to_string(),
            executor: Arc::new(crate::application::executor::CommandExecutor::new(
                Arc::new(crate::application::executor::FakeAdapter),
                16,
                4,
            )),
        }
    }

    /// `/get_config` serves both methods the frontend uses (boot + editor
    /// config loads POST, editor save flows GET). Regression: POST 405'd.
    // The guard serializes tests sharing the global config; it must span
    // the whole test (the route under test never takes this lock, so no
    // deadlock — clippy's await_holding_lock does not apply here).
    #[allow(clippy::await_holding_lock)]
    #[tokio::test]
    async fn get_config_accepts_get_and_post() {
        let _guard = config_guard();
        seed_config(&serde_json::json!({
            "url": {"port": 5000},
            "front": {"buttons": {}},
            "settings": {},
        }));
        for method in ["GET", "POST"] {
            let response = app_router(test_state())
                .oneshot(
                    Request::builder()
                        .method(method)
                        .uri("/get_config")
                        .header("host", "localhost:5000")
                        .extension(axum::extract::ConnectInfo(
                            "127.0.0.1:3000".parse::<SocketAddr>().unwrap(),
                        ))
                        .header("content-type", "application/json")
                        .body(Body::from("{}"))
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK, "{method} /get_config");
        }
    }
}
