//! Port of `app/utils/restart.py`.

use crate::app::utils::{exit::exit_program, logger::log};

/// Port of `restart_program`.
pub fn restart_program() {
    let result = restart_inner();
    if let Err(e) = result {
        log().exception(&e, Some("Error while restarting the program"), true, true, true);
    }
}

fn restart_inner() -> Result<(), String> {
    // Release builds behave like frozen runs: allow the new instance through
    // the single-instance guard, spawn it, then exit this one.
    if !cfg!(debug_assertions) {
        let content = std::fs::read_to_string("temp.json").map_err(|e| e.to_string())?;
        let mut temp_data: serde_json::Value =
            serde_json::from_str(&content).map_err(|e| e.to_string())?;
        temp_data["allow_multiple_instances"] = serde_json::Value::Bool(true);
        std::fs::write("temp.json", serde_json::to_string_pretty(&temp_data).unwrap())
            .map_err(|e| e.to_string())?;

        let exe =
            std::env::current_exe().map_err(|e| format!("Cannot locate executable: {e}"))?;
        std::process::Command::new(exe)
            .spawn()
            .map_err(|e| format!("Cannot respawn executable: {e}"))?;
        exit_program(true, false);
        return Ok(());
    }

    // Debug builds behave like unfrozen runs: re-exec this process.
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        let exe =
            std::env::current_exe().map_err(|e| format!("Cannot locate executable: {e}"))?;
        let args: Vec<String> = std::env::args().skip(1).collect();
        let err = std::process::Command::new(exe).args(args).exec();
        return Err(format!("Cannot re-exec process: {err}"));
    }

    #[cfg(windows)]
    {
        let exe =
            std::env::current_exe().map_err(|e| format!("Cannot locate executable: {e}"))?;
        let args: Vec<String> = std::env::args().skip(1).collect();
        std::process::Command::new(exe)
            .args(args)
            .spawn()
            .map_err(|e| format!("Cannot respawn executable: {e}"))?;
        exit_program(true, false);
        Ok(())
    }

    #[cfg(not(any(unix, windows)))]
    {
        Err("restart_program is not supported on this platform".to_string())
    }
}
