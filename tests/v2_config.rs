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
fn migration_preserves_extensions_order_and_has_protected_idempotent_backup() {
    let value = json!({"settings":{"extension":{"secret":"not-a-real-key"}}, "front":{"height":"1","width":"2","buttons":{"z":[{"message":"/custom <|§|>quoted path"}],"a":[{"image":"custom.svg"}]}},"plugin_data":[1,2]});
    let fixture = Fixture::new(value.clone());
    let service = fixture.service();
    let snapshot = service.snapshot().unwrap();
    assert_eq!(snapshot.config["schema_version"], 2);
    assert_eq!(snapshot.config["plugin_data"], value["plugin_data"]);
    let mut expected_settings = value["settings"].clone();
    expected_settings["v2_security"] = json!("legacy");
    assert_eq!(snapshot.config["settings"], expected_settings);
    assert_eq!(
        snapshot.config["front"]["buttons"]
            .as_object()
            .unwrap()
            .keys()
            .collect::<Vec<_>>(),
        vec!["z", "a"]
    );
    let backup = fixture.0.with_extension("v1.backup.json");
    assert_eq!(
        serde_json::from_slice::<Value>(&fs::read(&backup).unwrap()).unwrap(),
        value
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(&backup).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
    let persisted = fs::read(&fixture.0).unwrap();
    drop(service);
    assert_eq!(
        fixture.service().snapshot().unwrap().config,
        snapshot.config
    );
    assert_eq!(fs::read(&fixture.0).unwrap(), persisted);
}

#[test]
fn revisions_conflict_after_external_edits_and_invalid_edits_keep_last_valid_snapshot() {
    let fixture = Fixture::new(json!({"settings":{},"front":{"buttons":{}}}));
    let service = fixture.service();
    let first = service.snapshot().unwrap();
    fs::write(
        &fixture.0,
        br#"{"schema_version":2,"settings":{"external":true}}"#,
    )
    .unwrap();
    assert_eq!(
        service
            .replace(first.config, Some(first.revision))
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
    assert!(service.replace(json!({}), None).is_err());
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
fn concurrent_revision_updates_have_one_winner_and_legacy_transactions_do_not_lose_edits() {
    let fixture = Fixture::new(json!({"count":0}));
    let service = Arc::new(fixture.service());
    let revision = service.snapshot().unwrap().revision;
    let barrier = Arc::new(Barrier::new(3));
    let mut tasks = Vec::new();
    for _ in 0..2 {
        let (service, barrier) = (service.clone(), barrier.clone());
        tasks.push(std::thread::spawn(move || {
            barrier.wait();
            service.update(Some(revision), |mut value| {
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
    let tasks: Vec<_> = (0..8)
        .map(|_| {
            let service = service.clone();
            std::thread::spawn(move || {
                service
                    .update(None, |mut value| {
                        value["count"] = json!(value["count"].as_u64().unwrap() + 1);
                        Ok(value)
                    })
                    .unwrap()
            })
        })
        .collect();
    for task in tasks {
        task.join().unwrap();
    }
    assert_eq!(service.snapshot().unwrap().config["count"], 9);
}

#[test]
fn interrupted_transaction_preserves_external_file_and_rollback_restores_supported_data() {
    let fixture = Fixture::new(json!({"setting":"original"}));
    let service = fixture.service();
    let first = service.snapshot().unwrap();
    let updated = service
        .replace(json!({"setting":"changed"}), Some(first.revision))
        .unwrap();
    let restored = service.restore_backup(updated.revision).unwrap();
    assert_eq!(restored.config["setting"], "original");
    let error = service
        .update(Some(restored.revision), |_| {
            fs::write(&fixture.0, br#"{"schema_version":2,"setting":"external"}"#).unwrap();
            Ok(json!({"setting":"stale"}))
        })
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::Conflict);
    assert_eq!(service.snapshot().unwrap().config["setting"], "external");
}

#[cfg(unix)]
#[test]
fn failed_disk_write_preserves_file_and_snapshot() {
    use std::os::unix::fs::PermissionsExt;
    let fixture = Fixture::new(json!({"value":"old"}));
    let service = fixture.service();
    let before = service.snapshot().unwrap();
    let bytes = fs::read(&fixture.0).unwrap();
    let dir = fixture.0.parent().unwrap();
    fs::set_permissions(dir, fs::Permissions::from_mode(0o500)).unwrap();
    let result = service.replace(json!({"value":"new"}), Some(before.revision));
    fs::set_permissions(dir, fs::Permissions::from_mode(0o700)).unwrap();
    assert_eq!(result.unwrap_err().code, ErrorCode::PersistenceFailed);
    assert_eq!(service.last_valid().config, before.config);
    assert_eq!(fs::read(&fixture.0).unwrap(), bytes);
}

#[test]
fn independent_configuration_owners_serialize_writes_with_the_os_lock() {
    let fixture = Fixture::new(json!({"count":0}));
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
                service
                    .update(None, |mut value| {
                        let before = value["count"].as_u64().unwrap();
                        std::thread::sleep(std::time::Duration::from_millis(5));
                        value["count"] = json!(before + 1);
                        Ok(value)
                    })
                    .unwrap();
            })
        })
        .collect();
    for worker in workers {
        worker.join().unwrap();
    }
    assert_eq!(fixture.service().snapshot().unwrap().config["count"], 8);
}
