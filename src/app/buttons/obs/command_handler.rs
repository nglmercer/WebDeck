//! Port of `app/buttons/obs/command_handler.py`.
//!
//! Backend mapping: `obs-websocket-py` (sync) → [`obws`] 0.15 (async,
//! awaited through `utils::block_on`). Connect-per-command, the localized
//! connection-error mapping, and the subcommand routing are 1:1 with
//! Python — including the branch order (`/obs_toggle_rec` shadows
//! `/obs_toggle_rec_pause`, exactly as in Python).
//!
//! Documented deviations:
//! - Missing `obs_host`/`obs_port`/`obs_password` globals (fresh boot before
//!   any config save) heal via `reload_obs()` instead of failing with a
//!   `TypeError`-derived connection error like Python.
//! - The `"10061"` refused-check also matches `"Connection refused"` (same
//!   condition on non-Windows targets), and the password check also matches
//!   obws's authentication-failure text (`"4009"`, `"Authentication
//!   Failed"`); obs-websocket-py's `"password may be inco"` text is kept
//!   as a fallback predicate.
//! - `obs.disconnect()` is implicit: the obws client disconnects on drop,
//!   on every return path (Python skips it when a command raises —
//!   unobservable).

use obws::requests::hotkeys::KeyModifiers;
use serde_json::{json, Value};

use crate::app::buttons::obs::{
    recording, scenes,
    streaming::{self as stream},
    utils::{block_on, failure, reload_obs},
    virtualcam,
};
use crate::app::utils::{global_variables::get_global_variables, languages::text, logger::log};

/// Port of `handle_command`.
/// Reference: <https://github.com/obsproject/obs-websocket/blob/master/docs/generated/protocol.md>
pub fn handle_command(message: &str) -> Value {
    let [host, port, password] = get_global_variables(["obs_host", "obs_port", "obs_password"]);
    let (host, port, password) = match (host, port, password) {
        (Some(host), Some(port), Some(password)) => (
            host.as_str().unwrap_or("localhost").to_string(),
            port.as_u64().unwrap_or(4455) as u16,
            password.as_str().unwrap_or("").to_string(),
        ),
        // Fresh boot before any config save: heal from config (see docs).
        _ => {
            let session = reload_obs();
            (session.host, session.port, session.password)
        }
    };

    let client = match block_on(obws::Client::connect(
        host.as_str(),
        port,
        Some(password.as_str()),
    )) {
        Ok(client) => client,
        Err(e) => {
            let detail = format!("{e:?}");
            let error = if detail.contains("10061") || detail.contains("Connection refused") {
                log().exception(
                    &e,
                    Some("Failed connection to obs: The websocket server cannot be found."),
                    true,
                    false,
                    true,
                );
                text(Some("obs_error_10061"), None)
            } else if detail.contains("password may be inco")
                || detail.contains("4009")
                || detail.contains("Authentication Failed")
            {
                log().exception(
                    &e,
                    Some("Failed connection to obs: Password may be incorrect."),
                    true,
                    false,
                    true,
                );
                text(Some("obs_error_incorrect_password"), None)
            } else {
                e.to_string()
            };

            // Port of `raise ConnectionError(...)` (the route maps raises to
            // failure JSON).
            return failure(format!(
                "{}: {error}",
                text(Some("obs_failed_connection_error"), None).replace('.', "")
            ));
        }
    };

    let result = if message.starts_with("/obs_toggle_rec") {
        recording::toggle(&client)
    } else if message.starts_with("/obs_start_rec") {
        recording::start(&client)
    } else if message.starts_with("/obs_stop_rec") {
        recording::stop(&client)
    } else if message.starts_with("/obs_toggle_rec_pause") {
        recording::pause_toggle(&client)
    } else if message.starts_with("/obs_pause_rec") {
        recording::pause(&client)
    } else if message.starts_with("/obs_resume_rec") {
        recording::resume(&client)
    } else if message.starts_with("/obs_toggle_stream") {
        stream::toggle(&client)
    } else if message.starts_with("/obs_start_stream") {
        stream::start(&client)
    } else if message.starts_with("/obs_stop_stream") {
        stream::stop(&client)
    } else if message.starts_with("/obs_toggle_virtualcam") {
        virtualcam::toggle(&client)
    } else if message.starts_with("/obs_start_virtualcam") {
        virtualcam::start(&client)
    } else if message.starts_with("/obs_stop_virtualcam") {
        virtualcam::stop(&client)
    } else if message.starts_with("/obs_scene") {
        let scene_name = message.replace("/obs_scene", "");
        scenes::set(&client, scene_name.trim())
    } else if message.starts_with("/obs_key") {
        let hotkey = message.split(' ').next_back().unwrap_or("");
        match block_on(
            client
                .hotkeys()
                .trigger_by_sequence(&format!("OBS_KEY_{hotkey}"), KeyModifiers::default()),
        ) {
            Ok(()) => {
                log().success(&format!("Hotkey triggered '{hotkey}' successfully."));
                Ok(())
            }
            Err(e) => {
                log().error(&format!("Failed to trigger hotkey '{hotkey}': {e}"));
                Err(format!("{} :/", text(Some("failed"), None)))
            }
        }
    } else {
        Ok(())
    };

    // Port of `obs.disconnect()` (obws disconnects on drop).
    drop(client);

    match result {
        Ok(()) => json!({"success": true}),
        Err(message) => failure(message),
    }
}
