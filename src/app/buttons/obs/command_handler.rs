//! Port of `app/buttons/obs/command_handler.py`.
//!
//! Subcommand routing is ported 1:1; the wire calls are TODO via `obws`.

use serde_json::{json, Value};

use crate::app::buttons::obs::{
    recording, scenes,
    streaming::{self as stream},
    utils::reload_obs,
    virtualcam,
};
use crate::app::utils::{languages::text, logger::log};

/// Port of `handle_command`.
pub fn handle_command(message: &str) -> Value {
    // Python connects here and maps failures to localized errors; without the
    // wire backend every command reports the connection failure the same way.
    let session = reload_obs();
    let connected = false; // TODO(port): obws connect.
    if !connected {
        let error = text(Some("obs_error_10061"), None);
        log().debug(&format!(
            "Failed connection to obs ({}:{}): {error}",
            session.host, session.port
        ));
        return json!({"success": false, "message": error});
    }

    #[allow(unreachable_code)]
    if message.starts_with("/obs_toggle_rec") {
        recording::toggle(&session);
    } else if message.starts_with("/obs_start_rec") {
        recording::start(&session);
    } else if message.starts_with("/obs_stop_rec") {
        recording::stop(&session);
    } else if message.starts_with("/obs_toggle_rec_pause") {
        recording::pause_toggle(&session);
    } else if message.starts_with("/obs_pause_rec") {
        recording::pause(&session);
    } else if message.starts_with("/obs_resume_rec") {
        recording::resume(&session);
    } else if message.starts_with("/obs_toggle_stream") {
        stream::toggle(&session);
    } else if message.starts_with("/obs_start_stream") {
        stream::start(&session);
    } else if message.starts_with("/obs_stop_stream") {
        stream::stop(&session);
    } else if message.starts_with("/obs_toggle_virtualcam") {
        virtualcam::toggle(&session);
    } else if message.starts_with("/obs_start_virtualcam") {
        virtualcam::start(&session);
    } else if message.starts_with("/obs_stop_virtualcam") {
        virtualcam::stop(&session);
    } else if message.starts_with("/obs_scene") {
        let scene_name = message.replacen("/obs_scene", "", 1);
        scenes::set(&session, scene_name.trim());
    } else if message.starts_with("/obs_key") {
        let hotkey = message.split(' ').next_back().unwrap_or("");
        // TODO(port): TriggerHotkeyByKeySequence via obws.
        log().warning(&format!("obs hotkey {hotkey:?}: obws backend not ported yet"));
    }

    json!({"success": true})
}
