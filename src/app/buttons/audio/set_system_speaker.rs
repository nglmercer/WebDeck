//! Port of `app/buttons/audio/set_system_speaker.py`.
//!
//! TODO(port): default-speaker switching via the `windows` crate
//! (PolicyConfig / Endpoints API). Python's body is itself unfinished
//! (`# PAS FINI` at the call site).

use crate::app::utils::logger::log;

/// Port of `set_speakers_by_name` — stub.
pub fn set_speakers_by_name(speakers_name: &str) {
    log().warning(&format!(
        "set_speakers_by_name({speakers_name:?}): endpoint switching not ported yet"
    ));
}
