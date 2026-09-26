//! Port of `app/buttons/audio/set_system_mic.py`.
//!
//! TODO(port): default-microphone switching via the `windows` crate
//! (PolicyConfig / Endpoints API). Python's body is itself unfinished
//! (`# PAS FINI` at the call site).

use crate::app::utils::logger::log;

/// Port of `set_microphone_by_name` — stub.
pub fn set_microphone_by_name(name: &str) {
    log().warning(&format!(
        "set_microphone_by_name({name:?}): endpoint switching not ported yet"
    ));
}
