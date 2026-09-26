//! Port of `app/buttons/exec/batch_code.py`.
//!
//! Python's body is itself an unimplemented stub (`# TODO: execute batch
//! here`); mirrored as a stub.

use crate::app::utils::logger::log;

/// Port of `execute` — stub (upstream TODO).
pub fn execute(file_path: &str) {
    let content = std::fs::read_to_string(file_path).unwrap_or_default();
    log().warning(&format!(
        "exec batch {} ({} bytes): batch execution not implemented upstream or here",
        file_path,
        content.len()
    ));
}
