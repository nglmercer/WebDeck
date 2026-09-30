use serde_json::{json, Value};
use std::{
    fs,
    path::PathBuf,
    sync::{Arc, Barrier},
};
use webdeck::{application::config::ConfigService, domain::error::ErrorCode};

struct Fixture(PathBuf);
impl Fixture {
    fn new(value: Value) -> Self {
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!(
            "webdeck-v2-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.json");
        fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
        Self(path)
    }
    fn service(&self) -> ConfigService {
        ConfigService::open(self.0.clone()).unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(self.0.parent().unwrap());
    }
}

#[test]
fn missing_and_old_schemas_are_rejected_without_modification_or_backup() {
    for value in [
        json!({"settings":{}, "front":{}}),
        json!({"schema_version":1,"private":"keep"}),
    ] {
        let fixture = Fixture::new(value);
        let before = fs::read(&fixture.0).unwrap();
        let backup = fixture.0.with_extension("v1.backup.json");
        fs::write(&backup, b"existing backup").unwrap();
        assert!(
            matches!(ConfigService::open(fixture.0.clone()), Err(e) if e.code == ErrorCode::UnsupportedSchema)
        );
        assert_eq!(fs::read(&fixture.0).unwrap(), before);
        assert_eq!(fs::read(&backup).unwrap(), b"existing backup");
    }
}

#[test]
fn opening_v2_is_read_only_and_preserves_extensions() {
    let value = json!({"schema_version":2,"settings":{"extension":{"secret":"not-a-real-key"}},"front":{"height":1,"width":2,"buttons":{"z":[],"a":[]}},"plugin_data":[1,2]});
    let fixture = Fixture::new(value.clone());
    let before = fs::read(&fixture.0).unwrap();
    let service = fixture.service();
    assert_eq!(service.snapshot().unwrap().config, value);
    assert_eq!(fs::read(&fixture.0).unwrap(), before);
    assert!(!fixture.0.with_extension("v1.backup.json").exists());
}

#[test]
fn revisions_conflict_after_external_edits_and_invalid_edits_keep_last_valid_snapshot() {
    let fixture = Fixture::new(json!({"schema_version":2,"settings":{},"front":{"buttons":{}}}));
    let service = fixture.service();
    let first = service.snapshot().unwrap();
    fs::write(
        &fixture.0,
        br#"{"schema_version":2,"settings":{"external":true}}"#,
    )
    .unwrap();
    assert_eq!(
        service
            .replace(first.config, first.revision)
            .unwrap_err()
            .code,
        ErrorCode::Conflict
    );
    let valid = service.snapshot().unwrap();
    fs::write(&fixture.0, b"{broken").unwrap();
    assert_eq!(
        service.snapshot().unwrap_err().code,
        ErrorCode::InvalidInput
    );
    assert_eq!(service.last_valid().config, valid.config);
    assert!(service.replace(json!({}), valid.revision).is_err());
    assert_eq!(fs::read(&fixture.0).unwrap(), b"{broken");
}

#[test]
fn unsupported_schema_does_not_overwrite_or_back_up_unknown_data() {
    let fixture = Fixture::new(json!({"schema_version":999,"future":"preserve"}));
    let original = fs::read(&fixture.0).unwrap();
    assert!(
        matches!(ConfigService::open(fixture.0.clone()), Err(e) if e.code == ErrorCode::UnsupportedSchema)
    );
    assert_eq!(fs::read(&fixture.0).unwrap(), original);
    assert!(!fixture.0.with_extension("v1.backup.json").exists());
}

#[test]
fn concurrent_revision_updates_have_one_winner_and_explicit_transactions_preserve_edits() {
    let fixture = Fixture::new(json!({"schema_version":2,"count":0}));
    let service = Arc::new(fixture.service());
    let revision = service.snapshot().unwrap().revision;
    let barrier = Arc::new(Barrier::new(3));
    let mut tasks = Vec::new();
    for _ in 0..2 {
        let (service, barrier) = (service.clone(), barrier.clone());
        tasks.push(std::thread::spawn(move || {
            barrier.wait();
            service.update(revision, |mut value| {
                value["count"] = json!(value["count"].as_u64().unwrap() + 1);
                Ok(value)
            })
        }));
    }
    barrier.wait();
    let outcomes: Vec<_> = tasks.into_iter().map(|t| t.join().unwrap()).collect();
    assert_eq!(outcomes.iter().filter(|r| r.is_ok()).count(), 1);
    assert!(outcomes
        .iter()
        .any(|r| matches!(r, Err(e) if e.code == ErrorCode::Conflict)));
    for _ in 0..8 {
        let current = service.snapshot().unwrap();
        service
            .update(current.revision, |mut value| {
                value["count"] = json!(value["count"].as_u64().unwrap() + 1);
                Ok(value)
            })
            .unwrap();
    }
    assert_eq!(service.snapshot().unwrap().config["count"], 9);
}

#[test]
fn interrupted_transaction_preserves_external_file() {
    let fixture = Fixture::new(json!({"schema_version":2,"setting":"original"}));
    let service = fixture.service();
    let first = service.snapshot().unwrap();
    let updated = service
        .replace(
            json!({"schema_version":2,"setting":"changed"}),
            first.revision,
        )
        .unwrap();
    let error = service
        .update(updated.revision, |_| {
            fs::write(&fixture.0, br#"{"schema_version":2,"setting":"external"}"#).unwrap();
            Ok(json!({"schema_version":2,"setting":"stale"}))
        })
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::Conflict);
    assert_eq!(service.snapshot().unwrap().config["setting"], "external");
}

#[cfg(unix)]
#[test]
fn failed_disk_write_preserves_file_and_snapshot() {
    use std::os::unix::fs::PermissionsExt;
    let fixture = Fixture::new(json!({"schema_version":2,"value":"old"}));
    let service = fixture.service();
    let before = service.snapshot().unwrap();
    let bytes = fs::read(&fixture.0).unwrap();
    let dir = fixture.0.parent().unwrap();
    fs::set_permissions(dir, fs::Permissions::from_mode(0o500)).unwrap();
    let result = service.replace(json!({"schema_version":2,"value":"new"}), before.revision);
    fs::set_permissions(dir, fs::Permissions::from_mode(0o700)).unwrap();
    assert_eq!(result.unwrap_err().code, ErrorCode::PersistenceFailed);
    assert_eq!(service.last_valid().config, before.config);
    assert_eq!(fs::read(&fixture.0).unwrap(), bytes);
}

#[test]
fn independent_configuration_owners_serialize_writes_with_the_os_lock() {
    let fixture = Fixture::new(json!({"schema_version":2,"count":0}));
    let services: Vec<_> = (0..8)
        .map(|_| Arc::new(ConfigService::open(fixture.0.clone()).unwrap()))
        .collect();
    let barrier = Arc::new(Barrier::new(8));
    let workers: Vec<_> = services
        .into_iter()
        .map(|service| {
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                loop {
                    let revision = service.snapshot().unwrap().revision;
                    match service.update(revision, |mut value| {
                        let before = value["count"].as_u64().unwrap();
                        std::thread::sleep(std::time::Duration::from_millis(5));
                        value["count"] = json!(before + 1);
                        Ok(value)
                    }) {
                        Ok(_) => break,
                        Err(error) if error.code == ErrorCode::Conflict => continue,
                        Err(error) => panic!("Unexpected write failure: {error:?}"),
                    }
                }
            })
        })
        .collect();
    for worker in workers {
        worker.join().unwrap();
    }
    assert_eq!(fixture.service().snapshot().unwrap().config["count"], 8);
}
