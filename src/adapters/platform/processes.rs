//! Owned shell processes. Do not confuse request cancellation with process
//! termination: blocking callers retain admission until the process returns.
use std::io;
use std::process::{Child, Command};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

fn children() -> &'static Mutex<Vec<Child>> {
    static CHILDREN: OnceLock<Mutex<Vec<Child>>> = OnceLock::new();
    CHILDREN.get_or_init(|| Mutex::new(Vec::new()))
}

fn command(source: &str) -> Command {
    #[cfg(windows)]
    let mut command = {
        let mut c = Command::new("cmd");
        c.args(["/C", source]);
        c
    };
    #[cfg(not(windows))]
    let mut command = {
        let mut c = Command::new("sh");
        c.args(["-c", source]);
        c
    };
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    command
}

pub fn spawn(source: &str) -> io::Result<()> {
    let mut children = children().lock().unwrap_or_else(|p| p.into_inner());
    children.retain_mut(|child| {
        let exited = matches!(child.try_wait(), Ok(Some(_)));
        #[cfg(unix)]
        {
            !exited || unsafe { libc::kill(-(child.id() as i32), 0) == 0 }
        }
        #[cfg(not(unix))]
        {
            !exited
        }
    });
    // Bound detached application launches as well as executor work.
    if children.len() >= 64 {
        return Err(io::Error::other("Process capacity exhausted"));
    }
    children.push(command(source).spawn()?);
    Ok(())
}

fn stop(child: &mut Child) {
    #[cfg(unix)]
    unsafe {
        libc::kill(-(child.id() as i32), libc::SIGKILL);
    }
    #[cfg(windows)]
    {
        let _ = Command::new("taskkill")
            .args(["/PID", &child.id().to_string(), "/T", "/F"])
            .status();
    }
    let _ = child.kill();
    let _ = child.wait();
}

pub fn run(source: &str, timeout: Duration) -> io::Result<i64> {
    let mut child = command(source).spawn()?;
    let deadline = Instant::now() + timeout;
    loop {
        if let Some(status) = child.try_wait()? {
            // A synchronous shell owns its descendants even when the leader exits first.
            stop(&mut child);
            return Ok(status.code().unwrap_or(-1) as i64);
        }
        if Instant::now() >= deadline {
            stop(&mut child);
            return Err(io::Error::new(io::ErrorKind::TimedOut, "Process timed out"));
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

pub fn shutdown() {
    let mut children = children().lock().unwrap_or_else(|p| p.into_inner());
    for child in children.iter_mut() {
        stop(child);
    }
    children.clear();
}
