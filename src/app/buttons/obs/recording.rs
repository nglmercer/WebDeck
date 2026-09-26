//! Port of `app/buttons/obs/recording.py` — TODO via `obws`.

use crate::app::buttons::obs::utils::ObsSession;
use crate::app::utils::logger::log;

/// Port of `toggle`.
pub fn toggle(_session: &ObsSession) {
    log().warning("obs recording.toggle: obws backend not ported yet");
}

/// Port of `start`.
pub fn start(_session: &ObsSession) {
    log().warning("obs recording.start: obws backend not ported yet");
}

/// Port of `stop`.
pub fn stop(_session: &ObsSession) {
    log().warning("obs recording.stop: obws backend not ported yet");
}

/// Port of `pause_toggle`.
pub fn pause_toggle(_session: &ObsSession) {
    log().warning("obs recording.pause_toggle: obws backend not ported yet");
}

/// Port of `pause`.
pub fn pause(_session: &ObsSession) {
    log().warning("obs recording.pause: obws backend not ported yet");
}

/// Port of `resume`.
pub fn resume(_session: &ObsSession) {
    log().warning("obs recording.resume: obws backend not ported yet");
}
