//! Disposable process tests; never send input or access native devices.
#[cfg(unix)]
#[test]
fn shell_timeout_and_shutdown_terminate_owned_process_groups_even_after_the_leader_exits() {
    use std::{
        fs,
        time::{Duration, Instant},
    };
    use webdeck::adapters::platform::processes;
    let started = Instant::now();
    assert_eq!(
        processes::run("sleep 30", Duration::from_millis(30))
            .unwrap_err()
            .kind(),
        std::io::ErrorKind::TimedOut
    );
    assert!(started.elapsed() < Duration::from_secs(2));
    let path = std::env::temp_dir().join(format!("webdeck-owned-child-{}.pid", std::process::id()));
    processes::spawn(&format!(
        "sleep 30 & echo $! > '{}'",
        path.to_string_lossy().replace('\'', "'\"'\"'")
    ))
    .unwrap();
    let deadline = Instant::now() + Duration::from_secs(2);
    while !path.exists() {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(5));
    }
    let pid: i32 = fs::read_to_string(&path).unwrap().trim().parse().unwrap();
    std::thread::sleep(Duration::from_millis(30));
    // Reaping the exited shell must retain its still-running group.
    processes::spawn("true").unwrap();
    processes::shutdown();
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        let running = fs::read_to_string(format!("/proc/{pid}/stat"))
            .is_ok_and(|stat| !stat.split(") ").nth(1).unwrap_or("").starts_with('Z'));
        if !running {
            break;
        }
        assert!(Instant::now() < deadline, "owned child remained alive");
        std::thread::sleep(Duration::from_millis(5));
    }
    fs::remove_file(path).unwrap();
}
