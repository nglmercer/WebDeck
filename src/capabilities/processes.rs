use super::*;
mod launcher;
mod owner;
pub(super) use owner::{ManagedChild, ProcessOwner};

pub(super) fn stop(c: &mut Child) {
    #[cfg(unix)]
    unsafe {
        libc::kill(-(c.id() as i32), libc::SIGKILL);
    }
    #[cfg(windows)]
    {
        if let Ok(mut cleanup) = Process::new("taskkill")
            .args(["/PID", &c.id().to_string(), "/T", "/F"])
            .spawn()
        {
            let deadline = Instant::now() + Duration::from_secs(2);
            loop {
                match cleanup.try_wait() {
                    Ok(Some(_)) => break,
                    Ok(None) if Instant::now() < deadline => {
                        std::thread::sleep(Duration::from_millis(10));
                    }
                    _ => {
                        let _ = cleanup.kill();
                        let _ = cleanup.wait();
                        break;
                    }
                }
            }
        }
    }
    let _ = c.kill();
    let _ = c.wait();
}

pub fn open(s: &str) -> Result<()> {
    #[cfg(windows)]
    let mut command = Process::new("rundll32.exe");
    #[cfg(windows)]
    command.args(["url.dll,FileProtocolHandler", s]);
    #[cfg(not(windows))]
    let mut command = Process::new("xdg-open");
    #[cfg(not(windows))]
    command.arg(s);
    launcher::launch(command)?;
    Ok(())
}
pub(super) fn run_at(deadline: Instant, program: &str, args: &[&str]) -> Result<Value> {
    let budget = deadline.saturating_duration_since(Instant::now());
    if budget.is_zero() {
        return Err(Error::new(
            ErrorCode::ExecutionFailed,
            "Execution budget exhausted",
        ));
    }
    run_timeout(program, args, budget.min(Duration::from_secs(5)))
}
pub(super) fn run_timeout(program: &str, args: &[&str], timeout: Duration) -> Result<Value> {
    let mut command = Process::new(program);
    command.args(args);
    let status = ManagedChild::spawn(command)?.wait(timeout)?;
    if status.success() {
        Ok(json!({}))
    } else {
        Err(Error::new(
            ErrorCode::ExecutionFailed,
            "Process exited unsuccessfully",
        ))
    }
}

pub(super) fn kill(s: &str, deadline: Instant) -> Result<Value> {
    #[cfg(windows)]
    return run_at(deadline, "taskkill", &["/F", "/IM", s]);
    #[cfg(not(windows))]
    return run_at(deadline, "pkill", &["-x", "--", s]);
}

#[cfg(test)]
mod budget_tests {
    use super::*;
    #[test]
    fn expired_helper_budget_rejects_before_process_spawn() {
        let error = run_at(
            Instant::now() - Duration::from_secs(1),
            "webdeck-nonexistent-helper",
            &[],
        )
        .unwrap_err();
        assert_eq!(error.code, ErrorCode::ExecutionFailed);
        assert_eq!(error.message, "Execution budget exhausted");
    }
}
