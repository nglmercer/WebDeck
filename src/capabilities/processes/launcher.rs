use super::*;
use std::sync::{mpsc, OnceLock};
use tokio::sync::Semaphore;

/// Desktop handoffs survive native-owner shutdown. Only their direct launchers are reaped.
pub(super) fn launch(command: Process) -> Result<u32> {
    static CAPACITY: OnceLock<Arc<Semaphore>> = OnceLock::new();
    launch_with_capacity(
        command,
        CAPACITY
            .get_or_init(|| Arc::new(Semaphore::new(64)))
            .clone(),
    )
}
fn launch_with_capacity(command: Process, capacity: Arc<Semaphore>) -> Result<u32> {
    let permit = capacity.try_acquire_owned().map_err(|_| {
        Error::new(
            ErrorCode::CapacityExhausted,
            "Desktop launcher capacity exhausted",
        )
    })?;
    let (result, spawned) = mpsc::sync_channel(1);
    // Start the waiter before spawning the child. Thread creation failure cannot orphan a child.
    std::thread::Builder::new()
        .name("desktop-launcher".into())
        .spawn(move || {
            let _owned = permit;
            let mut command = command;
            match command.spawn() {
                Ok(mut child) => {
                    let _ = result.send(Ok(child.id()));
                    let _ = child.wait();
                }
                Err(_) => {
                    let _ = result.send(Err(Error::execution()));
                }
            }
        })
        .map_err(|_| Error::execution())?;
    spawned.recv().map_err(|_| Error::execution())?
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    #[tokio::test]
    async fn native_owner_shutdown_leaves_desktop_handoff_alive() {
        use std::os::{fd::OwnedFd, unix::net::UnixStream};
        let (input, keep_alive) = UnixStream::pair().unwrap();
        let capacity = Arc::new(Semaphore::new(1));
        let mut command = Process::new("sh");
        command.args(["-c", "cat >/dev/null"]);
        command.stdin(std::process::Stdio::from(OwnedFd::from(input)));
        let pid = launch_with_capacity(command, capacity.clone()).unwrap();
        ProcessOwner::default().shutdown();
        assert_eq!(unsafe { libc::kill(pid as i32, 0) }, 0);
        assert_eq!(capacity.available_permits(), 0);
        assert_eq!(
            launch_with_capacity(Process::new("sh"), capacity.clone())
                .unwrap_err()
                .code,
            ErrorCode::CapacityExhausted
        );
        drop(keep_alive);
        let permit = tokio::time::timeout(Duration::from_secs(2), capacity.acquire())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(unsafe { libc::kill(pid as i32, 0) }, -1);
        drop(permit);
    }
    #[tokio::test]
    async fn launcher_reaps_exit_and_releases_capacity() {
        let capacity = Arc::new(Semaphore::new(1));
        let mut command = Process::new("sh");
        command.args(["-c", "exit 0"]);
        let pid = launch_with_capacity(command, capacity.clone()).unwrap();
        let permit = tokio::time::timeout(Duration::from_secs(2), capacity.acquire())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(unsafe { libc::kill(pid as i32, 0) }, -1);
        assert_eq!(
            std::io::Error::last_os_error().raw_os_error(),
            Some(libc::ESRCH)
        );
        drop(permit);
    }
    #[tokio::test]
    async fn spawn_failure_returns_error_and_releases_capacity() {
        let capacity = Arc::new(Semaphore::new(1));
        assert!(launch_with_capacity(
            Process::new("webdeck-nonexistent-launcher-test"),
            capacity.clone()
        )
        .is_err());
        let permit = tokio::time::timeout(Duration::from_secs(2), capacity.acquire())
            .await
            .unwrap()
            .unwrap();
        drop(permit);
    }
}
