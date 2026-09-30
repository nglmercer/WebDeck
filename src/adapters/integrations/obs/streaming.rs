//! Port of `app/buttons/obs/streaming.py`.

use crate::adapters::integrations::obs::utils::block_on;
use crate::app::utils::{languages::text, logger::log};

/// Port of `toggle`.
pub fn toggle(client: &obws::Client) -> Result<(), String> {
    let result = block_on(client.streaming().toggle());
    // NOTE: Python logs success before checking the outcome; mirrored 1:1.
    log().success("Streaming toggled successfully.");
    match result {
        Ok(_) => Ok(()),
        Err(e) => {
            log().error(&format!("Failed to toggle streaming: {e}"));
            Err(format!("{} :/", text(Some("failed"), None)))
        }
    }
}

/// Port of `start`.
pub fn start(client: &obws::Client) -> Result<(), String> {
    let status = match block_on(client.streaming().status()) {
        Ok(status) => status,
        Err(e) => return Err(e.to_string()),
    };
    if status.active {
        log().notice("OBS is already streaming.");
        Err(format!("{} :/", text(Some("obs_already_streaming"), None)))
    } else {
        // Python ignores the StartStream result; mirrored 1:1.
        let _ = block_on(client.streaming().start());
        log().success("Stream started successfully.");
        Ok(())
    }
}

/// Port of `stop`.
pub fn stop(client: &obws::Client) -> Result<(), String> {
    let status = match block_on(client.streaming().status()) {
        Ok(status) => status,
        Err(e) => return Err(e.to_string()),
    };
    if status.active {
        // Python ignores the StopStream result; mirrored 1:1.
        let _ = block_on(client.streaming().stop());
        log().success("Stream stopped successfully.");
        Ok(())
    } else {
        log().notice("OBS is not streaming.");
        Err(format!("{} :/", text(Some("obs_not_streaming"), None)))
    }
}
