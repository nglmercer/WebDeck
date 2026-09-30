//! Regression: exit paths must terminate the process on every platform.
//!
//! `exit_program(force=true)` used to log and return on non-Windows, so
//! tray Exit, `--timeout`, the `exit` positional, and release restart
//! left the app running forever. The `exit` positional below exercises
//! the exact call the tray Exit item makes (`exit_program(true, false)`).

use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// Wait up to `secs` for `child` to exit; kill and panic on timeout.
fn wait_for_exit(
    child: &mut std::process::Child,
    what: &str,
    secs: u64,
) -> std::process::ExitStatus {
    let deadline = Instant::now() + Duration::from_secs(secs);
    loop {
        match child.try_wait().expect("try_wait") {
            Some(status) => return status,
            None if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(100));
            }
            None => {
                let _ = child.kill();
                panic!("webdeck `{what}` did not terminate within {secs}s");
            }
        }
    }
}

#[test]
fn exit_positional_terminates_process() {
    let exe = env!("CARGO_BIN_EXE_webdeck");
    let mut child = Command::new(exe)
        .arg("exit")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn webdeck");
    let status = wait_for_exit(&mut child, "exit", 15);
    assert!(
        status.success(),
        "webdeck `exit` should terminate cleanly, got: {status}"
    );
}

/// argparse exits with code 2 on any parse error (`SystemExit` bypasses
/// Python's `except Exception`); a bad invocation must not start the app
/// with defaults.
#[test]
fn invalid_args_exit_with_code_2() {
    let exe = env!("CARGO_BIN_EXE_webdeck");
    let mut child = Command::new(exe)
        .arg("--bogus-flag-zzz")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn webdeck");
    let status = wait_for_exit(&mut child, "--bogus-flag-zzz", 15);
    assert_eq!(
        status.code(),
        Some(2),
        "bad args should exit(2) like argparse, got: {status}"
    );
}

#[test]
fn ordered_space_separated_options_and_timeout_exit_gracefully_with_isolated_config() {
    let directory = std::env::temp_dir().join(format!("webdeck-cli-owned-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let mut config: serde_json::Value =
        serde_json::from_str(include_str!("../webdeck/config_default.json")).unwrap();
    config["settings"]["show_popup"] = false.into();
    config["settings"]["auto_updates"] = false.into();
    std::fs::write(directory.join("config.json"), config.to_string()).unwrap();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let mut child = Command::new(env!("CARGO_BIN_EXE_webdeck"))
        .args([
            "--host",
            "127.0.0.1",
            "--port",
            &port.to_string(),
            "--timeout",
            "1",
            "--no-tray",
            "--no-admin",
            "--no-auto-update",
            "--force-start",
        ])
        .env("WEBDECK_CONFIG_DIR", &directory)
        .env("WEBDECK_FAKE_EFFECTS", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    assert!(wait_for_exit(&mut child, "ordered options + graceful timeout", 15).success());
    std::fs::remove_dir_all(directory).unwrap();
}
