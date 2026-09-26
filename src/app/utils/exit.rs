//! Port of `app/utils/exit.py`.

use crate::app::utils::logger::log;

/// Port of `exit_program`.
///
/// Windows behavior mirrors Python (kill `nircmd.exe`, plus `webdeck.exe`
/// when forced, then exit). The WMI enumeration is approximated with
/// `taskkill` (same observable effect; a `windows`-crate WMI port can refine
/// this later). Non-Windows behavior mirrors Python exactly: `force` only
/// logs, `!force` terminates this process (exit code 130 ≈ SIGINT).
pub fn exit_program(force: bool, from_timeout: bool) {
    if from_timeout {
        log().info("Timeout reached. Exiting WebDeck...");
    } else {
        log().info("Exiting WebDeck...");
    }

    #[cfg(windows)]
    {
        let mut targets = vec!["nircmd.exe"];
        if force {
            targets.push("webdeck.exe");
            if cfg!(debug_assertions) {
                log().debug(
                    "Currently not running in a frozen state (not compiled). Exiting current process.",
                );
            }
        }
        for target in targets {
            log().debug(&format!("Stopping process: {target}"));
            match std::process::Command::new("taskkill")
                .args(["/f", "/IM", target])
                .output()
            {
                Ok(output) if output.status.success() => {
                    log().success(&format!("Process terminated successfully: {target}"));
                }
                Ok(output) => {
                    log().debug(&format!(
                        "taskkill for {target} exited with {}",
                        output.status
                    ));
                }
                Err(e) => {
                    log().exception(&e, Some(&format!("Failed to terminate process '{target}'")), true, true, false);
                }
            }
        }
        if force {
            std::process::exit(0);
        }
    }

    #[cfg(not(windows))]
    {
        if force {
            log().debug("Force exit on non-Windows: nothing to terminate (mirrors exit.py).");
            return;
        }
    }

    if !force {
        log().debug("Force exit not enabled. Exiting current process.");
        std::process::exit(130);
    }
}
