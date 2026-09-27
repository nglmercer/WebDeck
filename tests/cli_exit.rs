//! Regression: exit paths must terminate the process on every platform.
//!
//! `exit_program(force=true)` used to log and return on non-Windows, so
//! tray Exit, `--timeout`, the `exit` positional, and release restart
//! left the app running forever. The `exit` positional below exercises
//! the exact call the tray Exit item makes (`exit_program(true, false)`).

use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

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
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        match child.try_wait().expect("try_wait") {
            Some(status) => {
                assert!(
                    status.success(),
                    "webdeck `exit` should terminate cleanly, got: {status}"
                );
                return;
            }
            None if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(100));
            }
            None => {
                let _ = child.kill();
                panic!("webdeck `exit` did not terminate within 15s");
            }
        }
    }
}
