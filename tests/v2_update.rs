use std::io::{self, Write};
struct Temp(std::path::PathBuf);
impl Temp {
    fn path(&self) -> &std::path::Path {
        &self.0
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn tempdir() -> std::io::Result<Temp> {
    let mut random = [0u8; 8];
    getrandom::fill(&mut random).map_err(std::io::Error::other)?;
    let path = std::env::temp_dir().join(format!("webdeck-update-{}", digest(&random)));
    std::fs::create_dir(&path)?;
    Ok(Temp(path))
}
use webdeck::update::{digest, install, install_with, stage};
fn archive(extra: &[(&str, &[u8])]) -> Vec<u8> {
    let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    for (name, bytes) in [
        ("WebDeck/WebDeck", b"new".as_slice()),
        ("WebDeck/WebDeck.exe", b"new".as_slice()),
        (
            "WebDeck/webdeck/config_default.json",
            include_bytes!("../webdeck/config_default.json").as_slice(),
        ),
        (
            "WebDeck/webdeck/version.json",
            include_bytes!("../webdeck/version.json").as_slice(),
        ),
        ("WebDeck/frontend/dist/index.html", b"new ui".as_slice()),
    ]
    .into_iter()
    .chain(extra.iter().copied())
    {
        zip.start_file(
            name,
            zip::write::SimpleFileOptions::default().unix_permissions(0o755),
        )
        .unwrap();
        zip.write_all(bytes).unwrap();
    }
    zip.finish().unwrap().into_inner()
}
#[test]
fn verified_upgrade_and_rollback_preserve_user_config_and_original_permissions() {
    let root = tempdir().unwrap();
    let live = root.path().join("live");
    std::fs::create_dir(&live).unwrap();
    std::fs::create_dir(live.join(".config")).unwrap();
    std::fs::write(live.join(".config/config.json"), b"user data").unwrap();
    std::fs::write(live.join("WebDeck"), b"old").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(live.join("WebDeck"), std::fs::Permissions::from_mode(0o751))
            .unwrap();
    }
    let bytes = archive(&[]);
    let staged = stage(
        &bytes,
        &format!("sha256:{}", digest(&bytes)),
        &root.path().join("stage"),
    )
    .unwrap();
    let backup = root.path().join("backup");
    install(&staged, &live, &backup).unwrap();
    assert_eq!(std::fs::read(live.join("WebDeck")).unwrap(), b"new");
    assert_eq!(
        std::fs::read(live.join(".config/config.json")).unwrap(),
        b"user data"
    );
    let result = std::process::Command::new(env!("CARGO_BIN_EXE_update"))
        .arg("rollback")
        .arg("--backup")
        .arg(&backup)
        .arg("--destination")
        .arg(&live)
        .output()
        .unwrap();
    assert!(result.status.success());
    assert_eq!(std::fs::read(live.join("WebDeck")).unwrap(), b"old");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(live.join("WebDeck"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o751
        );
    }
    assert!(!live.join("frontend/dist/index.html").exists());
    assert_eq!(
        std::fs::read(live.join(".config/config.json")).unwrap(),
        b"user data"
    );
}
#[test]
fn interrupted_install_restores_all_originals_and_removes_new_files() {
    let root = tempdir().unwrap();
    let live = root.path().join("live");
    std::fs::create_dir(&live).unwrap();
    std::fs::write(live.join("WebDeck"), b"old").unwrap();
    let bytes = archive(&[]);
    let staged = stage(
        &bytes,
        &format!("sha256:{}", digest(&bytes)),
        &root.path().join("stage"),
    )
    .unwrap();
    let backup = root.path().join("backup");
    assert!(
        install_with(&staged, &live, &backup, |index, _| if index == 2 {
            Err(io::Error::other("interruption"))
        } else {
            Ok(())
        })
        .is_err()
    );
    assert_eq!(std::fs::read(live.join("WebDeck")).unwrap(), b"old");
    assert!(!live.join("WebDeck.exe").exists());
    assert!(backup.join("rolled-back").is_file());
}
#[test]
fn invalid_digest_traversal_and_config_replacement_never_publish() {
    let root = tempdir().unwrap();
    let bytes = archive(&[]);
    assert!(stage(&bytes, "sha256:wrong", &root.path().join("bad-digest")).is_err());
    for (i, name) in [
        "WebDeck/../outside",
        "WebDeck/.config/config.json",
        "WebDeck/static/../../outside",
        "WebDeck/static\\outside",
        "outside",
        "WebDeck/static/NUL",
        "WebDeck/static/NUL.txt",
        "WebDeck/static/COM1.txt",
        "WebDeck/static/lpt9.png",
    ]
    .iter()
    .enumerate()
    {
        let bytes = archive(&[(name, b"unsafe")]);
        let target = root.path().join(format!("invalid-{i}"));
        assert!(
            stage(&bytes, &format!("sha256:{}", digest(&bytes)), &target).is_err(),
            "{name}"
        );
        assert!(!target.exists());
    }
    assert!(!root.path().join("outside").exists());
}
#[cfg(unix)]
#[test]
fn installation_rejects_symlinked_destination_without_touching_target() {
    let root = tempdir().unwrap();
    let live = root.path().join("live");
    let outside = root.path().join("outside");
    std::fs::create_dir(&live).unwrap();
    std::fs::create_dir(&outside).unwrap();
    std::os::unix::fs::symlink(&outside, live.join("webdeck")).unwrap();
    let bytes = archive(&[]);
    let staged = stage(
        &bytes,
        &format!("sha256:{}", digest(&bytes)),
        &root.path().join("stage"),
    )
    .unwrap();
    assert!(install(&staged, &live, &root.path().join("backup")).is_err());
    assert!(!outside.join("version.json").exists());
}
#[test]
fn ordinary_download_without_trusted_https_source_is_denied() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    for url in [
        "http://github.com/file",
        "file:///tmp/file",
        "https://evil.example/file",
    ] {
        assert!(runtime.block_on(webdeck::update::download(url)).is_err());
    }
}

#[test]
#[ignore = "requires WEBDECK_PORTABLE_ARTIFACT from a native package build"]
fn portable_archive_upgrade_and_real_updater_rollback_preserve_user_data() {
    let artifact = std::env::var("WEBDECK_PORTABLE_ARTIFACT").expect("set artifact path");
    let bytes = std::fs::read(&artifact).unwrap();
    let expected = std::fs::read_to_string(format!("{artifact}.sha256")).unwrap();
    assert_eq!(digest(&bytes), expected.split_whitespace().next().unwrap());
    let root = tempdir().unwrap();
    let live = root.path().join("live");
    std::fs::create_dir_all(live.join(".config/user_uploads")).unwrap();
    std::fs::write(live.join(".config/config.json"), b"original user config").unwrap();
    std::fs::write(
        live.join(".config/user_uploads/image.png"),
        b"original upload",
    )
    .unwrap();
    let app = if cfg!(windows) {
        "WebDeck.exe"
    } else {
        "WebDeck"
    };
    std::fs::write(live.join(app), b"old application").unwrap();
    let staged = stage(
        &bytes,
        &format!("sha256:{}", digest(&bytes)),
        &root.path().join("stage"),
    )
    .unwrap();
    let backup = root.path().join("backup");
    install(&staged, &live, &backup).unwrap();
    assert!(std::fs::metadata(live.join(app)).unwrap().len() > 1024);
    assert!(live.join("frontend/dist/index.html").is_file());
    let updater = staged.join(if cfg!(windows) {
        "update.exe"
    } else {
        "update"
    });
    let result = std::process::Command::new(updater)
        .arg("rollback")
        .arg("--backup")
        .arg(&backup)
        .arg("--destination")
        .arg(&live)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(std::fs::read(live.join(app)).unwrap(), b"old application");
    assert!(!live.join("frontend/dist/index.html").exists());
    assert_eq!(
        std::fs::read(live.join(".config/config.json")).unwrap(),
        b"original user config"
    );
    assert_eq!(
        std::fs::read(live.join(".config/user_uploads/image.png")).unwrap(),
        b"original upload"
    );
}
#[test]
fn release_selection_uses_only_verified_newer_maintainer_v2_prereleases() {
    let platform = if cfg!(windows) { "windows" } else { "linux" };
    let url=format!("https://github.com/nglmercer/WebDeck/releases/download/v2.0.0-alpha.2/WebDeck-2.0.0-alpha.2-{platform}-{}-portable.zip",std::env::consts::ARCH);
    let release = serde_json::json!({"draft":false,"prerelease":true,"tag_name":"v2.0.0-alpha.2","assets":[{"browser_download_url":url,"digest":format!("sha256:{}","a".repeat(64))}]});
    assert_eq!(
        webdeck::update::select_release(std::slice::from_ref(&release))
            .unwrap()
            .unwrap()
            .version,
        "2.0.0-alpha.2"
    );
    for (field, value) in [
        ("draft", serde_json::json!(true)),
        ("prerelease", serde_json::json!(false)),
        ("tag_name", serde_json::json!("v1.9.0")),
    ] {
        let mut bad = release.clone();
        bad[field] = value;
        assert!(webdeck::update::select_release(&[bad]).unwrap().is_none());
    }
    for u in [
        url.replace("nglmercer", "other"),
        url.replace("alpha.2", "alpha.1"),
        url.replace("-portable.zip", "-dev-portable.zip"),
    ] {
        let mut bad = release.clone();
        bad["assets"][0]["browser_download_url"] = serde_json::json!(u);
        assert!(webdeck::update::select_release(&[bad]).unwrap().is_none());
    }
}
