//! Linux desktop helpers (extracted from `commands.rs`).
//!
//! The PC-control commands are `shutdown`/`rundll32`/`taskkill` one-liners on
//! Windows; on Linux they map to the systemd/logind/freedesktop/PipeWire
//! equivalents below. Every helper degrades to a log line when its tool is
//! missing (Wayland and minimal installs vary).

use super::spawn_shell;
use crate::app::utils::logger::log;

/// True when a CLI tool exists on PATH.
#[cfg(target_os = "linux")]
pub(crate) fn tool_present(tool: &str) -> bool {
    std::process::Command::new("sh")
        .args(["-c", &format!("command -v {tool} >/dev/null 2>&1")])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Run a shell command, returning exit success (for fallback chains —
/// unlike fire-and-forget `spawn_shell`).
#[cfg(target_os = "linux")]
pub(crate) fn run_shell_checked(command: &str) -> bool {
    std::process::Command::new("sh")
        .args(["-c", command])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Shell-quote one argument (single-quote wrapping).
#[cfg(target_os = "linux")]
pub(crate) fn shell_quote(arg: &str) -> String {
    format!("'{}'", arg.replace('\'', "'\\''"))
}

/// Call `org.freedesktop.ScreenSaver.<method>` (lock/activate/idle-reset),
/// trying qdbus6 → qdbus → gdbus.
#[cfg(target_os = "linux")]
pub(crate) fn screensaver_dbus(method: &str, extra_args: &str) -> bool {
    const DEST: &str = "org.freedesktop.ScreenSaver";
    const PATH: &str = "/ScreenSaver";
    if tool_present("qdbus6") || tool_present("qdbus") {
        let qdbus = if tool_present("qdbus6") {
            "qdbus6"
        } else {
            "qdbus"
        };
        return run_shell_checked(&format!(
            "{qdbus} {DEST} {PATH} {DEST}.{method} {extra_args}"
        ));
    }
    if tool_present("gdbus") {
        return run_shell_checked(&format!(
            "gdbus call --session --dest {DEST} --object-path {PATH} --method {DEST}.{method} {extra_args}"
        ));
    }
    log().warning("screensaver_dbus: no qdbus/qdbus6/gdbus on PATH");
    false
}

/// Send an MPRIS `Player.<command>` (`PlayPause`/`Previous`/`Next`) to the
/// first registered media player (port of the media-key presses).
#[cfg(target_os = "linux")]
pub(crate) fn mpris_command(command: &str) {
    // Player discovery via the session bus service list.
    let lister = if tool_present("qdbus6") {
        Some("qdbus6")
    } else if tool_present("qdbus") {
        Some("qdbus")
    } else {
        None
    };
    let mut players: Vec<String> = Vec::new();
    if let Some(lister) = lister {
        if let Ok(output) = std::process::Command::new(lister).output() {
            players = String::from_utf8_lossy(&output.stdout)
                .lines()
                .filter(|l| l.starts_with("org.mpris.MediaPlayer2."))
                .map(|l| l.trim().to_string())
                .collect();
        }
    }
    if players.is_empty() && tool_present("gdbus") {
        if let Ok(output) = std::process::Command::new("gdbus")
            .args([
                "call",
                "--session",
                "--dest",
                "org.freedesktop.DBus",
                "--object-path",
                "/",
                "--method",
                "org.freedesktop.DBus.ListNames",
            ])
            .output()
        {
            // Output shape: ([':1.1', 'org.mpris.MediaPlayer2.foo', ...],)
            players = String::from_utf8_lossy(&output.stdout)
                .split('\'')
                .filter(|s| s.starts_with("org.mpris.MediaPlayer2."))
                .map(|s| s.to_string())
                .collect();
        }
    }
    let Some(player) = players.into_iter().next() else {
        log().warning(&format!(
            "mpris {command}: no media player on the session bus"
        ));
        return;
    };
    let sender = if tool_present("qdbus6") {
        format!("qdbus6 {player} /org/mpris/MediaPlayer2 org.mpris.MediaPlayer2.Player.{command}")
    } else if tool_present("qdbus") {
        format!("qdbus {player} /org/mpris/MediaPlayer2 org.mpris.MediaPlayer2.Player.{command}")
    } else {
        format!(
            "gdbus call --session --dest {player} --object-path /org/mpris/MediaPlayer2 \
             --method org.mpris.MediaPlayer2.Player.{command}"
        )
    };
    if !run_shell_checked(&sender) {
        log().warning(&format!("mpris {command}: send to {player} failed"));
    }
}

/// Open the desktop's screen-locker settings (KDE/GNOME best effort).
#[cfg(target_os = "linux")]
pub(crate) fn linux_screensaver_settings() {
    let desktop = std::env::var("XDG_CURRENT_DESKTOP")
        .unwrap_or_default()
        .to_lowercase();
    if desktop.contains("kde") && tool_present("systemsettings") {
        spawn_shell("systemsettings kcm_screenlocker");
    } else if desktop.contains("gnome") && tool_present("gnome-control-center") {
        spawn_shell("gnome-control-center screen");
    } else if tool_present("systemsettings") {
        spawn_shell("systemsettings kcm_screenlocker");
    } else if tool_present("gnome-control-center") {
        spawn_shell("gnome-control-center screen");
    } else {
        log().warning("/screensaversettings: no systemsettings or gnome-control-center");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_os = "linux")]
    #[test]
    fn shell_quote_wraps_safely() {
        assert_eq!(shell_quote("firefox"), "'firefox'");
        assert_eq!(shell_quote("a'b"), "'a'\\''b'");
    }
}
