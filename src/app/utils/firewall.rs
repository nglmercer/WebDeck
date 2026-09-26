//! Port of `app/utils/firewall.py`.

use crate::app::utils::logger::log;

/// Port of `fix_firewall_permission`.
pub fn fix_firewall_permission() {
    #[cfg(windows)]
    {
        let exe = std::env::current_exe()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        let status = std::process::Command::new("powershell")
            .args([
                "-NoProfile",
                "New-NetFirewallRule",
                "-DisplayName",
                "\"WebDeck\"",
                "-Direction",
                "Inbound",
                "-Program",
                &format!("\"{exe}\""),
                "-Action",
                "Allow",
            ])
            .status();
        match status {
            Ok(status) if status.success() => {}
            other => log().debug(&format!("Firewall rule command result: {other:?}")),
        }
        log().info("Firewall permission should be fixed.");
    }
    #[cfg(not(windows))]
    {
        log().warning("fix_firewall_permission is only supported on Windows.");
    }
}

/// Port of `check_firewall_permission`.
///
/// Python queries `HNetCfg.FwMgr` over COM; this port checks for the WebDeck
/// rule via `netsh` (same question, no COM dependency). Like Python, any
/// error assumes permission is granted (returns `true`).
pub fn check_firewall_permission() -> bool {
    #[cfg(windows)]
    {
        match std::process::Command::new("netsh")
            .args(["advfirewall", "firewall", "show", "rule", "name=WebDeck"])
            .output()
        {
            Ok(output) if output.status.success() => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let has_rule = stdout.lines().count() > 3 && !stdout.contains("No rules match");
                if has_rule {
                    log().debug("The application has permission to pass through the firewall.");
                } else {
                    log().debug("The application does not have permission to pass through the firewall.");
                }
                has_rule
            }
            Ok(_) => {
                log().debug("The application does not have permission to pass through the firewall.");
                false
            }
            Err(e) => {
                log().exception(&e, Some("Error checking firewall permissions."), true, true, true);
                true
            }
        }
    }
    #[cfg(not(windows))]
    {
        true
    }
}
