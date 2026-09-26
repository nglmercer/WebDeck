//! Port of `app/buttons/obs/scenes.py` — TODO via `obws`.

use crate::app::buttons::obs::utils::ObsSession;
use crate::app::utils::logger::log;

/// Port of `set`.
pub fn set(_session: &ObsSession, scene_name: &str) {
    log().warning(&format!("obs scene.set({scene_name:?}): obws backend not ported yet"));
}
