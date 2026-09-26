//! Port of `app/buttons/obs/utils.py`.
//!
//! TODO(port): live connection via `obws` (obs-websocket, replacing
//! `obswebsocket`). Until then [`reload_obs`] reads the same config keys and
//! returns the session parameters; calls that need the wire protocol log and
//! report failure the same way connection errors surface in Python.

use crate::app::utils::{logger::log, settings::get_config::get_config};

/// OBS connection parameters — port of the
/// `(obs_host, obs_port, obs_password, obs)` tuple from `reload_obs()`.
/// (The live client field lands with the `obws` backend.)
#[derive(Debug, Clone)]
pub struct ObsSession {
    pub host: String,
    pub port: u16,
    pub password: String,
}

/// Port of `reload_obs`.
pub fn reload_obs() -> ObsSession {
    let config = get_config(false, false);
    let obs = &config["settings"]["obs"];
    let session = ObsSession {
        host: obs.get("host").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
        port: obs
            .get("port")
            .and_then(|v| v.as_u64().or_else(|| v.as_str().and_then(|s| s.parse().ok())))
            .unwrap_or(4455) as u16,
        password: obs.get("password").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
    };
    log().debug(&format!(
        "OBS session reloaded: {}:{} (wire protocol via obws not ported yet)",
        session.host, session.port
    ));
    session
}
