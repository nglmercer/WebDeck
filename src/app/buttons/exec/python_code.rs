//! Port of `app/buttons/exec/python_code.py`.
//!
//! Python `exec()`s the file content. There is no safe in-process equivalent
//! in Rust; planned replacement is an embedded `rhai` script engine with a
//! WebDeck API shim (see `docs/MIGRATION_RUST.md`).

use crate::app::utils::logger::log;

/// Port of `execute` — stub.
pub fn execute(file_path: &str) {
    let content = std::fs::read_to_string(file_path).unwrap_or_default();
    log().warning(&format!(
        "exec python {} ({} bytes): script execution not ported yet (rhai planned)",
        file_path,
        content.len()
    ));
}
