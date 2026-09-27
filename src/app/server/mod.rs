//! Port of `app/server.py` — Flask → axum.
//!
//! Route table (same paths + methods as the Flask app):
//! `GET /`, `GET /api/boot`, `POST /usage`, `POST /save_config`,
//! `POST /COMPLETE_save_config`, `POST /save_single_button`,
//! `POST /save_buttons_only`, `GET /get_config`, `POST /upload_folderpath`,
//! `POST /upload_filepath`, `POST /upload_file`, `POST /create_folder`,
//! `GET /.config/<dir>/<file>`, `POST /send-data`, plus `/static/*` (Flask
//! static folder) and `/assets/*` (TypeScript frontend bundle).
//!
//! Mapping notes:
//! - `@app.before_request check_local_network` → [`check_local_network`]
//!   middleware. One intentional deviation: Python tests
//!   `remote_ip in ipaddress.ip_address(network)` for `allowed_networks`,
//!   which raises `TypeError` (an `Address` is not a container); Rust
//!   implements the evidently intended CIDR-contains semantics.
//! - `@app.after_request` → [`after_request`] middleware (skips `/usage`).
//! - `@app.errorhandler(Exception)` → [`internal_error`] (always JSON; the
//!   `flask_debug` HTML-fallthrough is a dev-only path and is not mirrored).
//! - `render_template("index.jinja")` is replaced by the TypeScript SPA in
//!   `frontend/` (custom zero-dependency framework, 1:1 port). `GET /`
//!   serves its bundle; `GET /api/boot` returns the old template context
//!   (plus `lang`, `audio_devices`, `dark_theme`) as JSON.
//! - Flask-SocketIO (`connect`/`send`/`message_from_socket`) is ported via
//!   `socketioxide` ([`socketio_layer`]); the emit points live in the
//!   `handle_command` callers.
//! - `werkzeug` vs `app.run` selection collapses to one axum backend.

mod assets;
mod middleware;
mod realtime;
mod routes_boot;
mod routes_config;
mod routes_upload;

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
use serde_json::{json, Value};
use tokio::sync::Mutex;
use tower_http::services::ServeDir;

use self::middleware::{after_request, check_local_network};
use self::realtime::{send_data_route, socketio_layer, usage};
use self::routes_boot::{boot, home};
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

/// Shared server state — port of the module-level `config`,
/// `folders_to_create`, and `local_ip` globals in `app/server.py`.
/// (Config itself is re-read from disk per handler, exactly like Python.)
#[derive(Clone)]
pub struct AppState {
    pub folders_to_create: Arc<Mutex<Vec<Value>>>,
    pub local_ip: String,
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

/// Port of `run_server`.
///
/// Python runs `on_start()` + firewall checks at import time; Rust runs them
/// at the top of this function (same order, same conditions).
pub async fn run_server() -> Result<(), ServerError> {
    let (config, _commands, local_ip) = on_start().await;
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
        folders_to_create: Arc::new(Mutex::new(Vec::new())),
        local_ip: local_ip.clone(),
    };

    let app = Router::new()
        .route("/", get(home))
        .route("/api/boot", get(boot))
        .route("/usage", post(usage))
        .route("/save_config", post(saveconfig))
        .route("/COMPLETE_save_config", post(complete_save_config))
        .route("/save_single_button", post(save_single_button))
        .route("/save_buttons_only", post(save_buttons_only))
        .route("/get_config", get(get_config_route))
        .route("/upload_folderpath", post(upload_folderpath))
        .route("/upload_filepath", post(upload_filepath))
        .route("/upload_file", post(upload_file))
        .route("/create_folder", post(create_folder))
        .route("/.config/{directory}/{filename}", get(get_config_file))
        .route("/send-data", post(send_data_route))
        .nest_service("/static", ServeDir::new("static"))
        .nest_service("/assets", ServeDir::new("frontend/dist/assets"))
        .layer(axum_middleware::from_fn_with_state(
            state.clone(),
            check_local_network,
        ))
        .layer(DefaultBodyLimit::disable())
        .with_state(state)
        .layer(axum_middleware::from_fn(after_request));

    let host = get_args().host.clone().unwrap_or(local_ip);
    let port = get_port();
    let app = app.layer(socketio_layer(&host, port));
    let listener = tokio::net::TcpListener::bind(format!("{host}:{port}"))
        .await
        .map_err(|e| ServerError(format!("Cannot bind {host}:{port}: {e}")))?;
    log().info(&format!("Serving on {host}:{port}"));
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .map_err(|e| ServerError(format!("Server failed: {e}")))?;
    Ok(())
}
