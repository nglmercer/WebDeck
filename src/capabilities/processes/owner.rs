use super::*;

/// Only children explicitly launched by WebDeck belong to this owner.
#[derive(Default)]
pub(in crate::capabilities) struct ProcessOwner {
    state: Mutex<State>,
}
#[derive(Default)]
struct State {
    closed: bool,
    children: Vec<ManagedChild>,
}
impl ProcessOwner {
    pub(in crate::capabilities) fn spawn(&self, command: Process) -> Result<Value> {
        let mut state = self.state.lock().unwrap_or_else(|p| p.into_inner());
        if state.closed {
            return Err(Error::new(
                ErrorCode::ShuttingDown,
                "Process owner is closed",
            ));
        }
        // Dropping completed leaders also closes their remaining owned process group.
        state
            .children
            .retain_mut(|child| matches!(child.child.try_wait(), Ok(None)));
        if state.children.len() >= 64 {
            return Err(Error::new(
                ErrorCode::CapacityExhausted,
                "Process capacity exhausted",
            ));
        }
        state.children.push(ManagedChild::spawn(command)?);
        Ok(json!({"launched":true}))
    }
    pub(in crate::capabilities) fn shutdown(&self) {
        let children = {
            let mut state = self.state.lock().unwrap_or_else(|p| p.into_inner());
            state.closed = true;
            std::mem::take(&mut state.children)
        };
        // Kill/wait outside the ownership lock. No spawn can race the closed state.
        drop(children);
    }
}
impl Drop for ProcessOwner {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// Guarantees cleanup on success, timeout, wait errors, and unwinding.
pub(in crate::capabilities) struct ManagedChild {
    child: Child,
}
impl ManagedChild {
    /// Captures known system helpers with bounded pipes and one total deadline.
    #[cfg(any(target_os = "linux", all(test, unix)))]
    pub(in crate::capabilities) fn output(
        mut command: Process,
        timeout: Duration,
    ) -> Result<std::process::Output> {
        use std::process::Stdio;
        use std::sync::mpsc;
        let deadline = Instant::now() + timeout;
        command
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = Self::spawn(command)?;
        let stdout = child.child.stdout.take().ok_or_else(Error::execution)?;
        let stderr = child.child.stderr.take().ok_or_else(Error::execution)?;
        let (send, receive) = mpsc::channel();
        for (index, pipe, limit) in [
            (0, Box::new(stdout) as Box<dyn Read + Send>, 1024 * 1024),
            (1, Box::new(stderr) as Box<dyn Read + Send>, 64 * 1024),
        ] {
            let send = send.clone();
            std::thread::Builder::new()
                .name("native-output".into())
                .spawn(move || {
                    let mut bytes = Vec::new();
                    let result = pipe
                        .take(limit + 1)
                        .read_to_end(&mut bytes)
                        .map_err(|_| Error::execution())
                        .and_then(|_| {
                            if bytes.len() as u64 > limit {
                                Err(Error::new(
                                    ErrorCode::ExecutionFailed,
                                    "Process output limit exceeded",
                                ))
                            } else {
                                Ok(bytes)
                            }
                        });
                    let _ = send.send((index, result));
                })
                .map_err(|_| Error::execution())?;
        }
        drop(send);
        let mut pipes = [Vec::new(), Vec::new()];
        for _ in 0..2 {
            let (index, result) = receive
                .recv_timeout(deadline.saturating_duration_since(Instant::now()))
                .map_err(|_| Error::new(ErrorCode::ExecutionFailed, "Process output timed out"))?;
            pipes[index] = result?;
        }
        let status = child.wait(deadline.saturating_duration_since(Instant::now()))?;
        let [stdout, stderr] = pipes;
        Ok(std::process::Output {
            status,
            stdout,
            stderr,
        })
    }
    pub(in crate::capabilities) fn spawn(mut command: Process) -> Result<Self> {
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            command.process_group(0);
        }
        let child = command.spawn().map_err(|_| Error::execution())?;
        Ok(Self { child })
    }
    pub(in crate::capabilities) fn wait(
        &mut self,
        timeout: Duration,
    ) -> Result<std::process::ExitStatus> {
        let deadline = Instant::now() + timeout;
        loop {
            if let Some(status) = self.child.try_wait().map_err(|_| Error::execution())? {
                return Ok(status);
            }
            if Instant::now() >= deadline {
                return Err(Error::new(ErrorCode::ExecutionFailed, "Process timed out"));
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}
impl Drop for ManagedChild {
    fn drop(&mut self) {
        stop(&mut self.child);
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    fn capture(source: &str, timeout: Duration) -> Result<std::process::Output> {
        let mut command = Process::new("sh");
        command.args(["-c", source]);
        ManagedChild::output(command, timeout)
    }
    #[test]
    fn captures_both_pipes_and_preserves_exit_status() {
        let out = capture(
            "printf hello; printf problem >&2; exit 7",
            Duration::from_secs(2),
        )
        .unwrap();
        assert_eq!(out.stdout, b"hello");
        assert_eq!(out.stderr, b"problem");
        assert_eq!(out.status.code(), Some(7));
    }
    #[test]
    fn excessive_output_fails_without_waiting_for_eof() {
        let error = capture("yes", Duration::from_secs(2)).unwrap_err();
        assert_eq!(error.message, "Process output limit exceeded");
        let error = capture("yes >&2", Duration::from_secs(2)).unwrap_err();
        assert_eq!(error.message, "Process output limit exceeded");
    }
    #[test]
    fn stalled_pipe_obeys_total_deadline() {
        let error = capture("sleep 30", Duration::from_millis(20)).unwrap_err();
        assert_eq!(error.message, "Process output timed out");
    }
    fn sleeper() -> Process {
        let mut command = Process::new("sh");
        command.args(["-c", "sleep 30"]);
        command
    }
    #[test]
    fn timeout_reaps_owned_child() {
        let mut child = ManagedChild::spawn(sleeper()).unwrap();
        let id = child.child.id() as i32;
        assert!(child.wait(Duration::from_millis(20)).is_err());
        drop(child);
        assert_eq!(unsafe { libc::kill(id, 0) }, -1);
        assert_eq!(
            std::io::Error::last_os_error().raw_os_error(),
            Some(libc::ESRCH)
        );
    }
    #[test]
    fn shutdown_is_idempotent_and_rejects_later_spawns() {
        let owner = ProcessOwner::default();
        owner.spawn(sleeper()).unwrap();
        let id = owner.state.lock().unwrap().children[0].child.id() as i32;
        owner.shutdown();
        owner.shutdown();
        assert_eq!(unsafe { libc::kill(id, 0) }, -1);
        assert_eq!(
            owner.spawn(sleeper()).unwrap_err().code,
            ErrorCode::ShuttingDown
        );
    }
    #[test]
    fn dropping_owner_reaps_live_children() {
        let owner = ProcessOwner::default();
        owner.spawn(sleeper()).unwrap();
        let id = owner.state.lock().unwrap().children[0].child.id() as i32;
        drop(owner);
        assert_eq!(unsafe { libc::kill(id, 0) }, -1);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn shutdown_terminates_owned_descendants() {
        struct Ready(std::path::PathBuf);
        impl Drop for Ready {
            fn drop(&mut self) {
                let _ = fs::remove_file(&self.0);
            }
        }
        let ready = Ready(std::env::temp_dir().join(format!(
            "webdeck-descendant-{}",
            crate::domain::id().unwrap()
        )));
        let mut command = Process::new("sh");
        command.args([
            "-c",
            "sleep 30 & printf '%s' \"$!\" > \"$1\"; wait",
            "webdeck-test",
        ]);
        command.arg(&ready.0);
        let owner = ProcessOwner::default();
        owner.spawn(command).unwrap();
        let deadline = Instant::now() + Duration::from_secs(2);
        let descendant: i32 = loop {
            if let Ok(value) = fs::read_to_string(&ready.0) {
                if let Ok(id) = value.parse() {
                    break id;
                }
            }
            assert!(
                Instant::now() < deadline,
                "descendant did not signal readiness"
            );
            std::thread::sleep(Duration::from_millis(5));
        };
        owner.shutdown();
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            let status = fs::read_to_string(format!("/proc/{descendant}/status"));
            // An orphan zombie is terminated; its reaping belongs to the host's init.
            if status.is_err()
                || status
                    .unwrap()
                    .lines()
                    .any(|line| line.starts_with("State:\tZ"))
            {
                break;
            }
            assert!(Instant::now() < deadline, "owned descendant still running");
            std::thread::sleep(Duration::from_millis(5));
        }
    }
}
