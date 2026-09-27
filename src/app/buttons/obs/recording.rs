//! Port of `app/buttons/obs/recording.py`.

use crate::app::buttons::obs::utils::block_on;
use crate::app::utils::{languages::text, logger::log};

/// Port of `toggle`.
pub fn toggle(client: &obws::Client) -> Result<(), String> {
    let result = block_on(client.recording().toggle());
    // NOTE: Python logs success before checking the outcome; mirrored 1:1.
    log().success("Recording toggled successfully.");
    match result {
        Ok(_) => Ok(()),
        Err(e) => {
            log().error(&format!("Failed to toggle recording: {e}"));
            Err(format!("{} :/", text(Some("failed"), None)))
        }
    }
}

/// Port of `start`.
pub fn start(client: &obws::Client) -> Result<(), String> {
    let status = match block_on(client.recording().status()) {
        Ok(status) => status,
        Err(e) => return Err(e.to_string()),
    };
    if status.active {
        log().notice("OBS is already recording.");
        Err(text(Some("obs_already_recording"), None))
    } else {
        // Python ignores the StartRecord result; mirrored 1:1.
        let _ = block_on(client.recording().start());
        log().success("Recording started successfully.");
        Ok(())
    }
}

/// Port of `stop`.
pub fn stop(client: &obws::Client) -> Result<(), String> {
    let status = match block_on(client.recording().status()) {
        Ok(status) => status,
        Err(e) => return Err(e.to_string()),
    };
    if status.active {
        // Python ignores the StopRecord result; mirrored 1:1.
        let _ = block_on(client.recording().stop());
        log().success("Recording stopped successfully.");
        Ok(())
    } else {
        log().notice("OBS is not recording.");
        Err(text(Some("obs_not_recording"), None))
    }
}

/// Port of `pause_toggle`.
pub fn pause_toggle(client: &obws::Client) -> Result<(), String> {
    let result = block_on(client.recording().toggle_pause());
    log().success("Play/pause toggled successfully.");
    match result {
        Ok(_) => Ok(()),
        Err(e) => {
            log().error(&format!("Failed to toggle play/pause: {e}"));
            Err(format!("{} :/", text(Some("failed"), None)))
        }
    }
}

/// Port of `pause`.
pub fn pause(client: &obws::Client) -> Result<(), String> {
    let status = match block_on(client.recording().status()) {
        Ok(status) => status,
        Err(e) => return Err(e.to_string()),
    };
    if !status.active {
        log().notice("OBS is not recording, cannot pause.");
        return Err(text(Some("obs_no_recording_can_be_paused"), None));
    }

    match block_on(client.recording().pause()) {
        Ok(()) => Ok(()),
        Err(e) => {
            log().error(&format!(
                "Failed to pause recording. It might be that the recording is already paused or no recording is currently active. Details: {e}"
            ));
            Err(text(Some("obs_no_recording_can_be_paused"), None))
        }
    }
}

/// Port of `resume`.
///
/// TODO: Ensure consistency with the pause() function by checking if OBS is
/// recording before attempting to resume (kept from the Python source).
pub fn resume(client: &obws::Client) -> Result<(), String> {
    match block_on(client.recording().resume()) {
        Ok(()) => Ok(()),
        Err(e) => {
            log().error(&format!(
                "Failed to unpause recording. It might be that the recording is already active or no recording is currently active and paused. Details: {e}"
            ));
            Err(text(Some("obs_no_recording_is_paused"), None))
        }
    }
}
