//! Port of `app/buttons/exec/python_code.py`.
//!
//! Python `exec()`s the file content as **Python** in-process. Embedding a
//! Python interpreter is out of scope for the migration, so `/exec` scripts
//! run as **rhai** instead (same host API as `.rhai` plugins: logging,
//! `webdeck_command`, `run_shell`). This is the one deliberate language
//! change in the port; the dispatch, threading, and error surfacing stay 1:1.

use crate::app::utils::{logger::log, plugins::load_plugins::script_engine};

/// Port of `execute`: run a script file's content.
///
/// Returns `Err(message)` where Python raises (the route maps raises to
/// failure JSON).
pub fn execute(file_path: &str) -> Result<(), String> {
    let content = std::fs::read_to_string(crate::adapters::files::resolve_legacy_asset(file_path))
        .map_err(|e| e.to_string())?;
    run_script(&content).map_err(|e| e.to_string())
}

/// Run rhai source with the WebDeck host API (shared with single-line
/// `/exec`). Errors are logged like an uncaught thread traceback.
pub fn run_script(source: &str) -> Result<(), Box<rhai::EvalAltResult>> {
    let engine = script_engine();
    if let Err(e) = engine.run(source) {
        log().exception(&e, Some("Error while executing script"), true, true, true);
        return Err(e);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scripts_run_with_host_api() {
        assert!(run_script(r#"log_info("hi"); let x = 1 + 2; x"#).is_ok());
        assert!(run_script("webdeck_command(\"/debug-send\")").is_ok());
    }

    #[test]
    fn script_errors_surface() {
        assert!(run_script("this is not valid rhai +++").is_err());
        assert!(execute("/nonexistent-script-file.rhai").is_err());
    }
}
