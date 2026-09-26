//! Port of `app/buttons/soundboard/mic.py`.
//!
//! TODO(port): mic→VB-Cable loopback via `cpal` (+ `rubato` resampling),
//! replacing the `pyaudio` stream pump.

use crate::app::utils::logger::log;

/// Port of `soundboard` (the mic loop) — stub.
pub fn soundboard() {
    log().warning("soundboard mic loop: cpal backend not ported yet");
}

/// Port of `stop`.
pub fn stop() {
    log().debug("soundboard mic stop (backend not ported yet)");
}

/// Port of `restart`.
pub fn restart() {
    log().debug("soundboard mic restart (backend not ported yet)");
}
