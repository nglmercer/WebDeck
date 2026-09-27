//! Port of `app/utils/exit.py`.

use crate::app::utils::logger::log;

/// Port of `exit_program`.
///
/// Windows behavior mirrors Python (kill `nircmd.exe`, plus `webdeck.exe`
/// when forced, then exit). The WMI enumeration is approximated with
/// `taskkill` (same observable effect; a `windows`-crate WMI port can refine
/// this later). Non-Windows: `!force` terminates this process (exit code
/// 130 ≈ SIGINT, like Python's `os.kill(pid, SIGINT)`); `force` also
/// terminates it (exit code 0, like the Windows force path).
///
/// DELIBERATE DEVIATION: Python's `exit.py` returns without doing anything
/// for `force=True` off Windows. That was unobservable there — every
/// `force=True` caller (tray Exit, updater, restart, timeout) only runs on
/// Windows — but the Rust tray runs on Linux too, so returning would leave
/// the app (and `--timeout` / the `exit` positional / release restart)
/// running forever after asking to quit.
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
                    log().exception(
                        &e,
                        Some(&format!("Failed to terminate process '{target}'")),
                        true,
                        true,
                        false,
                    );
                }
            }
        }
        if force {
            std::process::exit(0);
        }
    }

    #[cfg(not(windows))]
    {
        // No separate kill step exists here (unlike WMI/taskkill on
        // Windows), so a forced exit terminates this process directly.
        if force {
            log().debug("Force exit on non-Windows: terminating current process.");
            std::process::exit(0);
        }
    }

    if !force {
        log().debug("Force exit not enabled. Exiting current process.");
        std::process::exit(130);
    }
}
