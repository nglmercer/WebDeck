//! Port of `app/buttons/obs/virtualcam.py`.

use crate::adapters::integrations::obs::utils::block_on;
use crate::app::utils::{languages::text, logger::log};

/// Port of `toggle`.
pub fn toggle(client: &obws::Client) -> Result<(), String> {
    let result = block_on(client.virtual_cam().toggle());
    // NOTE: Python logs success before checking the outcome; mirrored 1:1.
    log().success("Virtual cam toggled successfully.");
    match result {
        Ok(_) => Ok(()),
        Err(e) => {
            log().error(&format!("Failed to toggle virtual cam: {e}"));
            Err(format!("{} :/", text(Some("failed"), None)))
        }
    }
}

/// Port of `start`.
///
/// NOTE: Python calls `log.debug("obs recording_status: ", recording_status)`
/// with two arguments while `Logger.debug` takes one, so `/obs_start_virtualcam`
/// always crashes in Python. The port logs the same two values correctly
/// (this is the only intentional behavior fix in the OBS backend).
pub fn start(client: &obws::Client) -> Result<(), String> {
    let active = match block_on(client.virtual_cam().status()) {
        Ok(active) => active,
        Err(e) => return Err(e.to_string()),
    };
    log().debug(&format!("obs recording_status: {active}"));
    if active {
        log().notice("Virtual cam is already started.");
        Err(format!("{} :/", text(Some("obs_already_vcam"), None)))
    } else {
        // Python ignores the StartVirtualCam result; mirrored 1:1.
        let _ = block_on(client.virtual_cam().start());
        log().success("Virtual cam started successfully.");
        Ok(())
    }
}

/// Port of `stop`.
pub fn stop(client: &obws::Client) -> Result<(), String> {
    let active = match block_on(client.virtual_cam().status()) {
        Ok(active) => active,
        Err(e) => return Err(e.to_string()),
    };
    if active {
        // Python ignores the StopVirtualCam result; mirrored 1:1.
        let _ = block_on(client.virtual_cam().stop());
        log().success("Virtual cam stopped successfully.");
        Ok(())
    } else {
        log().notice("Virtual cam is already stopped.");
        Err(format!("{} :/", text(Some("obs_no_vcam"), None)))
    }
}
