//! Port of `app/buttons/obs/virtualcam.py` — TODO via `obws`.

use crate::app::buttons::obs::utils::ObsSession;
use crate::app::utils::logger::log;

/// Port of `toggle`.
pub fn toggle(_session: &ObsSession) {
    log().warning("obs virtualcam.toggle: obws backend not ported yet");
}

/// Port of `start`.
pub fn start(_session: &ObsSession) {
    log().warning("obs virtualcam.start: obws backend not ported yet");
}

/// Port of `stop`.
pub fn stop(_session: &ObsSession) {
    log().warning("obs virtualcam.stop: obws backend not ported yet");
}
