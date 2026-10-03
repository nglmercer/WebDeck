use super::*;

pub(super) fn power(action: &str, _deadline: Instant) -> Result<Value> {
    #[cfg(target_os = "linux")]
    return if action == "lock" {
        run_at(_deadline, "loginctl", &["lock-session"])
    } else {
        run_at(_deadline, "systemctl", &[action])
    };
    #[cfg(windows)]
    return match action {
        "poweroff" => run_at(_deadline, "shutdown", &["/s", "/f", "/t", "0"]),
        "reboot" => run_at(_deadline, "shutdown", &["/r", "/f", "/t", "0"]),
        "suspend" => run_at(
            _deadline,
            "rundll32.exe",
            &["powrprof.dll,SetSuspendState", "0,1,0"],
        ),
        "hibernate" => run_at(_deadline, "shutdown", &["/h"]),
        "lock" => run_at(_deadline, "rundll32.exe", &["user32.dll,LockWorkStation"]),
        _ => Err(unsupported()),
    };
    #[allow(unreachable_code)]
    Err(unsupported())
}

pub(super) fn screensaver_settings(_deadline: Instant) -> Result<Value> {
    #[cfg(windows)]
    return run_at(
        _deadline,
        "rundll32.exe",
        &["desk.cpl,InstallScreenSaver", "toasters.scr"],
    );
    #[cfg(target_os = "linux")]
    return run_at(_deadline, "systemsettings", &["kcm_screenlocker"]);
    #[allow(unreachable_code)]
    Err(unsupported())
}
pub(super) fn screensaver(mode: &str, _deadline: Instant) -> Result<Value> {
    #[cfg(target_os = "linux")]
    return match mode {
        "display_off" => run_at(_deadline, "xset", &["dpms", "force", "off"]),
        "start" => run_at(_deadline, "loginctl", &["lock-session"]),
        "stop" => run_at(
            _deadline,
            "dbus-send",
            &[
                "--session",
                "--dest=org.freedesktop.ScreenSaver",
                "/ScreenSaver",
                "org.freedesktop.ScreenSaver.SimulateUserActivity",
            ],
        ),
        _ => Err(Error::invalid()),
    };
    #[cfg(windows)]
    return match mode {
        "start" => run_at(_deadline, "scrnsave.scr", &["/s"]),
        "stop" => key(&["ctrl".into()], _deadline),
        "display_off" => windows_display_off(),
        _ => Err(Error::invalid()),
    };
    #[allow(unreachable_code)]
    Err(unsupported())
}
pub(super) fn firewall(_deadline: Instant) -> Result<Value> {
    #[cfg(windows)]
    {
        let exe = std::env::current_exe().map_err(|_| Error::execution())?;
        run_at(
            _deadline,
            "netsh",
            &[
                "advfirewall",
                "firewall",
                "add",
                "rule",
                "name=WebDeck v2",
                "dir=in",
                "action=allow",
                &format!("program={}", exe.display()),
                "enable=yes",
            ],
        )
    }
    #[cfg(not(windows))]
    Err(unsupported())
}
