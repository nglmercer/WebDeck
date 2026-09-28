//! Regression: `webdeck-qr` CLI contract.
//!
//! Every case below exits before any window opens, so this suite needs
//! no display. Window behavior itself is covered by the `utils::qr` unit
//! tests (frame composition, button geometry, hit-testing).

use std::process::{Command, Stdio};

fn qr() -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_webdeck-qr"));
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::null());
    cmd.stderr(Stdio::null());
    cmd
}

#[test]
fn help_exits_zero() {
    let status = qr().arg("--help").status().expect("spawn webdeck-qr");
    assert!(
        status.success(),
        "webdeck-qr --help should exit 0, got: {status}"
    );
}

#[test]
fn missing_png_fails_without_window() {
    let status = qr()
        .arg("--png=/nonexistent-webdeck-qr.png")
        .status()
        .expect("spawn webdeck-qr");
    assert_eq!(
        status.code(),
        Some(1),
        "missing --png should exit 1, got: {status}"
    );
}

#[test]
fn png_and_text_conflict() {
    let status = qr()
        .args(["--png=a.png", "--text=b"])
        .status()
        .expect("spawn webdeck-qr");
    assert_eq!(
        status.code(),
        Some(1),
        "--png + --text should exit 1, got: {status}"
    );
}

#[test]
fn neither_source_fails() {
    let status = qr().status().expect("spawn webdeck-qr");
    assert_eq!(
        status.code(),
        Some(1),
        "no source should exit 1, got: {status}"
    );
}

#[test]
fn invalid_size_exits_2() {
    let status = qr()
        .args(["--text=x", "--size=bogus"])
        .status()
        .expect("spawn webdeck-qr");
    assert_eq!(
        status.code(),
        Some(2),
        "bad --size should exit 2, got: {status}"
    );
}
