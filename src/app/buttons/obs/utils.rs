//! Port of `app/buttons/obs/utils.py`.
//!
//! Backend mapping: `obs-websocket-py` → [`obws`] 0.15 (async obs-websocket
//! client). [`reload_obs`] reads the same config keys and publishes the
//! same globals; the live connection is established per command in
//! `command_handler` (like Python's connect-per-command), with results
//! awaited through [`block_on`] since button handling is sync.

use serde_json::{json, Value};

use crate::app::utils::{global_variables::set_global_variable, settings::get_config::get_config};

/// OBS connection parameters — port of the
/// `(obs_host, obs_port, obs_password, obs)` tuple from `reload_obs()`.
/// (The live `obws::Client` connects per command; see `command_handler`.)
#[derive(Debug, Clone)]
pub struct ObsSession {
    pub host: String,
    pub port: u16,
    pub password: String,
}

/// Run an async OBS call from sync command context (mirrors
/// obs-websocket-py's blocking API). Safe from `spawn_blocking` threads
/// (no ambient runtime).
pub fn block_on<F: std::future::Future>(future: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("obs: cannot build tokio runtime")
        .block_on(future)
}

/// Port of the Flask route's exception mapping: a raise with message `m`
/// becomes `{"success": false, "message": m}` (`send_data_route` catches).
pub fn failure(message: String) -> Value {
    json!({"success": false, "message": message})
}

/// Port of `reload_obs`.
pub fn reload_obs() -> ObsSession {
    let config = get_config(false, false);
    let obs = &config["settings"]["obs"];
    let session = ObsSession {
        host: obs
            .get("host")
            .and_then(|v| v.as_str())
            .unwrap_or("localhost")
            .to_string(),
        port: obs
            .get("port")
            .and_then(|v| v.as_u64().or_else(|| v.as_str().and_then(|s| s.parse().ok())))
            .unwrap_or(4455) as u16,
        password: obs
            .get("password")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
    };

    set_global_variable("obs_host", json!(session.host));
    set_global_variable("obs_port", json!(session.port));
    set_global_variable("obs_password", json!(session.password));

    session
}
