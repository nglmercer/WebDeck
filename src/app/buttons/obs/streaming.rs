//! Port of `app/buttons/obs/streaming.py` — TODO via `obws`.

use crate::app::buttons::obs::utils::ObsSession;
use crate::app::utils::logger::log;

/// Port of `toggle`.
pub fn toggle(_session: &ObsSession) {
    log().warning("obs streaming.toggle: obws backend not ported yet");
}

/// Port of `start`.
pub fn start(_session: &ObsSession) {
    log().warning("obs streaming.start: obws backend not ported yet");
}

/// Port of `stop`.
pub fn stop(_session: &ObsSession) {
    log().warning("obs streaming.stop: obws backend not ported yet");
}
