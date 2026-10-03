use axum::{
    body::Body,
    extract::ConnectInfo,
    http::{Request, StatusCode},
};
use serde_json::{json, Value};
use std::{path::PathBuf, sync::Arc};
use tower::ServiceExt;
use webdeck::{
    contracts::*,
    domain,
    executor::{Adapter, Context, Executor},
    server::{router, App},
    sessions::Sessions,
    storage::{Assets, ConfigStore},
};
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!("webdeck-test-{}", domain::id().unwrap()));
        std::fs::create_dir(&p).unwrap();
        Self(p)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn store(t: &Temp) -> Arc<ConfigStore> {
    Arc::new(ConfigStore::open(t.0.join("config.json")).unwrap())
}
struct NoEffects;
impl Adapter for NoEffects {
    fn execute(&self, _: &Command, _: &Context) -> domain::Result<Value> {
        Ok(json!({"fake":true}))
    }
}
fn app(t: &Temp) -> App {
    App {
        io: Arc::new(tokio::sync::Semaphore::new(4)),
        queries: Arc::new(tokio::sync::Semaphore::new(2)),
        authorization: Arc::new(tokio::sync::Semaphore::new(4)),
        port: 5000,
        config: store(t),
        sessions: Arc::new(Sessions::open(t.0.join("devices.json")).unwrap()),
        executor: Arc::new(Executor::new(Arc::new(NoEffects), 2)),
        assets: Assets { root: t.0.clone() },
        plugins: Arc::new(Vec::new()),
    }
}
async fn call(
    a: &App,
    method: &str,
    path: &str,
    body: Value,
    remote: bool,
    token: Option<&str>,
    host: &str,
) -> axum::response::Response {
    let mut r = Request::builder()
        .method(method)
        .uri(path)
        .header("host", host)
        .header("content-type", "application/json");
    if let Some(t) = token {
        r = r.header("authorization", format!("Bearer {t}"));
    }
    let mut r = r.body(Body::from(body.to_string())).unwrap();
    r.extensions_mut().insert(ConnectInfo(
        if remote {
            "192.168.1.10:35000"
        } else {
            "127.0.0.1:35000"
        }
        .parse::<std::net::SocketAddr>()
        .unwrap(),
    ));
    router(a.clone()).oneshot(r).await.unwrap()
}
#[test]
fn configuration_validates_full_shape_and_preserves_extensions() {
    let c: Config = serde_json::from_str(include_str!("../webdeck/config_default.json")).unwrap();
    domain::validate_config(&c).unwrap();
    let mut c = c.clone();
    c.layout.folders[0].buttons[0]
        .extensions
        .insert("my.plugin".into(), json!({"x":true}));
    domain::validate_config(&c).unwrap();
    let mut invalid = c.clone();
    invalid.layout.folders[0].buttons[1].id = invalid.layout.folders[0].buttons[0].id.clone();
    assert!(domain::validate_config(&invalid).is_err());
    assert!(domain::decode_config(br#"{"settings":{},"front":{}}"#).is_err());
}
#[test]
fn old_layout_with_v2_marker_is_rejected_and_preserved() {
    let t = Temp::new();
    let path = t.0.join("config.json");
    let bytes = br#"{"schema_version":2,"settings":{},"front":{},"url":{}}"#;
    std::fs::write(&path, bytes).unwrap();
    let error = ConfigStore::open(path.clone()).err().unwrap();
    assert_eq!(error.code, ErrorCode::UnsupportedSchema);
    assert!(error.message.contains(path.to_str().unwrap()));
    assert!(error.message.contains("--config-dir"));
    assert_eq!(std::fs::read(path).unwrap(), bytes);
}
#[test]
fn disk_conflicts_and_invalid_reload_leave_last_valid_and_user_bytes() {
    let t = Temp::new();
    let s = store(&t);
    let first = s.snapshot().unwrap();
    let changed = s
        .mutate(first.revision, |c| {
            c.layout.columns = 5;
            Ok(())
        })
        .unwrap();
    assert_ne!(first.revision, changed.revision);
    assert_eq!(
        s.mutate(first.revision, |_| Ok(())).unwrap_err().code,
        ErrorCode::Conflict
    );
    let bytes = b"invalid json";
    std::fs::write(t.0.join("config.json"), bytes).unwrap();
    assert!(s.snapshot().is_err());
    assert_eq!(s.last_valid().revision, changed.revision);
    assert_eq!(std::fs::read(t.0.join("config.json")).unwrap(), bytes);
}
#[test]
fn paired_identity_has_no_invalid_credential_local_fallback() {
    let t = Temp::new();
    let s = Sessions::open(t.0.join("devices.json")).unwrap();
    assert!(s.authorize(None, true).is_ok());
    assert!(s.authorize(Some("invalid"), true).is_err());
    assert!(s.authorize(None, false).is_err());
    let approved = s
        .approve(DeviceRequest {
            name: "Phone".into(),
            capabilities: vec![Capability::Read, Capability::Input],
            ttl_seconds: 60,
        })
        .unwrap();
    assert_eq!(
        s.authorize(Some(&approved.token), false).unwrap(),
        approved.device.capabilities
    );
    s.revoke(&approved.device.id).unwrap();
    assert!(s.authorize(Some(&approved.token), false).is_err());
}
#[tokio::test]
async fn protected_routes_retired_protocol_and_dns_rebinding() {
    let t = Temp::new();
    let a = app(&t);
    assert_eq!(
        call(
            &a,
            "GET",
            "/api/v2/boot",
            json!(null),
            true,
            None,
            "192.168.1.2:5000"
        )
        .await
        .status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        call(
            &a,
            "GET",
            "/api/v2/boot",
            json!(null),
            false,
            Some("invalid"),
            "localhost:5000"
        )
        .await
        .status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        call(
            &a,
            "GET",
            "/api/v2/boot",
            json!(null),
            false,
            None,
            "evil.example:5000"
        )
        .await
        .status(),
        StatusCode::FORBIDDEN
    );
    for p in [
        "/send-data",
        "/save_config",
        "/get_config",
        "/api/boot",
        "/upload_file",
        "/.config/config.json",
    ] {
        assert_eq!(
            call(&a, "POST", p, json!({}), false, None, "localhost:5000")
                .await
                .status(),
            StatusCode::NOT_FOUND
        );
    }
    let granted = a
        .sessions
        .approve(DeviceRequest {
            name: "Phone".into(),
            capabilities: vec![Capability::Read, Capability::Input],
            ttl_seconds: 60,
        })
        .unwrap();
    let r = call(
        &a,
        "GET",
        "/api/v2/boot",
        json!(null),
        true,
        Some(&granted.token),
        "192.168.1.2:5000",
    )
    .await;
    assert_eq!(r.status(), StatusCode::OK);
    let b = axum::body::to_bytes(r.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let v: Value = serde_json::from_slice(&b).unwrap();
    assert!(v.get("settings").is_none());
    assert_eq!(v["can_edit"], false);
    assert_eq!(
        call(
            &a,
            "GET",
            "/api/v2/config",
            json!(null),
            true,
            Some(&granted.token),
            "192.168.1.2:5000"
        )
        .await
        .status(),
        StatusCode::FORBIDDEN
    );
}
#[tokio::test]
async fn accepted_work_survives_cancellation_and_capacity_counts_waiters() {
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Barrier,
    };
    struct Block {
        start: Arc<Barrier>,
        release: Arc<Barrier>,
        count: Arc<AtomicUsize>,
    }
    impl Adapter for Block {
        fn execute(&self, _: &Command, _: &Context) -> domain::Result<Value> {
            self.count.fetch_add(1, Ordering::SeqCst);
            self.start.wait();
            self.release.wait();
            Ok(json!({}))
        }
    }
    let start = Arc::new(Barrier::new(2));
    let release = Arc::new(Barrier::new(2));
    let count = Arc::new(AtomicUsize::new(0));
    let e = Arc::new(Executor::new(
        Arc::new(Block {
            start: start.clone(),
            release: release.clone(),
            count: count.clone(),
        }),
        1,
    ));
    let owner = e.clone();
    let task = tokio::spawn(async move {
        owner
            .execute(
                CommandRequest {
                    request_id: "first".into(),
                    command: Command::PlayPause,
                },
                vec![Capability::Audio],
                || {},
            )
            .await
    });
    tokio::task::spawn_blocking(move || start.wait())
        .await
        .unwrap();
    task.abort();
    assert_eq!(
        e.execute(
            CommandRequest {
                request_id: "second".into(),
                command: Command::PlayPause
            },
            vec![Capability::Audio],
            || {}
        )
        .await
        .unwrap_err()
        .code,
        ErrorCode::CapacityExhausted
    );
    tokio::task::spawn_blocking(move || release.wait())
        .await
        .unwrap();
    e.drain().await;
    assert_eq!(count.load(Ordering::SeqCst), 1);
}
#[test]
fn uploaded_assets_reject_escape_and_external_sources_are_explicit() {
    let t = Temp::new();
    let a = Assets { root: t.0.clone() };
    for p in ["..", "../config.json", "a/b", "a\\b", ":"] {
        assert!(a.path(p).is_err());
    }
    let s = a.upload("png", b"test").unwrap();
    let FileSource::Asset { id } = s else {
        panic!()
    };
    assert_eq!(a.read(&id).unwrap(), b"test");
    assert!(a
        .resolve(&FileSource::External {
            path: "relative/path".into()
        })
        .is_err());
}
#[test]
fn independent_session_owners_keep_grants_and_observe_revocation() {
    let t = Temp::new();
    let a = Sessions::open(t.0.join("devices.json")).unwrap();
    let b = Sessions::open(t.0.join("devices.json")).unwrap();
    let r = || DeviceRequest {
        name: "Controller".into(),
        capabilities: vec![Capability::Read],
        ttl_seconds: 60,
    };
    let first = a.approve(r()).unwrap();
    let second = b.approve(r()).unwrap();
    assert!(a.authorize(Some(&second.token), false).is_ok());
    b.revoke(&first.device.id).unwrap();
    assert!(a.authorize(Some(&first.token), false).is_err());
}
#[tokio::test]
async fn scripts_and_plugins_obey_nested_policy_and_budget_without_native_effects() {
    use webdeck::native::Native;
    let t = Temp::new();
    std::fs::create_dir(t.0.join("plugins")).unwrap();
    let manifest = json!({"schema_version":2,"id":"example","version":"2.0.0","entry":"example.rhai","actions":[{"id":"echo","label":"Echo","capabilities":["read"],"arguments":{"text":{"type":"string","required":true}},"result":{"type":"object","required":true}}]});
    std::fs::write(t.0.join("plugins/example.json"), manifest.to_string()).unwrap();
    std::fs::write(
        t.0.join("plugins/example.rhai"),
        r#"fn invoke_action(action,args){invoke(#{type:"debug",data:#{text:args.text}})}"#,
    )
    .unwrap();
    let native = Arc::new(
        Native::new(
            store(&t),
            Assets { root: t.0.clone() },
            Arc::new(tokio::sync::Notify::new()),
        )
        .unwrap(),
    );
    let executor = Executor::new(Arc::new(native), 1);
    let r = || CommandRequest {
        request_id: "test".into(),
        command: Command::Plugin {
            plugin_id: "example".into(),
            version: "2.0.0".into(),
            action_id: "echo".into(),
            args: std::collections::BTreeMap::from([("text".into(), json!("hello"))]),
        },
    };
    let result = executor
        .execute(r(), vec![Capability::Read, Capability::Plugin], || {})
        .await
        .unwrap();
    assert_eq!(result["value"]["data"]["text"], "hello");
    assert_eq!(
        executor
            .execute(r(), vec![Capability::Plugin], || {})
            .await
            .unwrap_err()
            .code,
        ErrorCode::Forbidden
    );
    let forbidden = CommandRequest {
        request_id: "nested".into(),
        command: Command::Script {
            source: ScriptSource::Inline {
                code: r#"invoke(#{type:"write",text:"never type this",send:false})"#.into(),
            },
        },
    };
    assert!(executor
        .execute(forbidden, vec![Capability::Script], || {})
        .await
        .is_err());
    let looped = CommandRequest {
        request_id: "loop".into(),
        command: Command::Script {
            source: ScriptSource::Inline {
                code: "loop {}".into(),
            },
        },
    };
    assert!(executor
        .execute(looped, vec![Capability::Script], || {})
        .await
        .is_err());
    executor.drain().await;
}
#[tokio::test]
async fn controller_boot_redacts_inline_action_secrets_and_both_http_actions_check_resolved_capabilities(
) {
    let t = Temp::new();
    let a = app(&t);
    let s = a.config.snapshot().unwrap();
    a.config
        .mutate(s.revision, |c| {
            c.settings.obs.password = "integration-secret".into();
            c.layout.extensions.insert(
                "appearance".into(),
                json!({"gap":0,"show_labels":false,"secret":"presentation-secret"}),
            );
            c.layout.folders[0].buttons[0].extensions.insert(
                "appearance".into(),
                json!({"columns":2,"secret":"presentation-secret"}),
            );
            c.layout.folders[0].buttons[0].action = ButtonAction::Command {
                command: Command::Fetch {
                    method: "GET".into(),
                    url: "https://example.test".into(),
                    headers: std::collections::BTreeMap::from([(
                        "Authorization".into(),
                        "action-secret".into(),
                    )]),
                    body: String::new(),
                    timeout_seconds: 1,
                },
            };
            Ok(())
        })
        .unwrap();
    let token = a
        .sessions
        .approve(DeviceRequest {
            name: "Controller".into(),
            capabilities: vec![Capability::Read, Capability::Input],
            ttl_seconds: 60,
        })
        .unwrap()
        .token;
    let r = call(
        &a,
        "GET",
        "/api/v2/boot",
        json!(null),
        true,
        Some(&token),
        "192.168.1.2:5000",
    )
    .await;
    let bytes = axum::body::to_bytes(r.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let text = String::from_utf8(bytes.to_vec()).unwrap();
    assert!(!text.contains("integration-secret"));
    assert!(!text.contains("action-secret"));
    assert!(!text.contains("presentation-secret"));
    let v: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(
        v["layout"]["extensions"]["appearance"],
        json!({"gap":0,"show_labels":false})
    );
    assert_eq!(
        v["layout"]["folders"][0]["buttons"][0]["extensions"]["appearance"],
        json!({"columns":2})
    );
    assert_eq!(
        v["layout"]["folders"][0]["buttons"][0]["action"]["command"],
        json!({"type":"button","button_id":"media"})
    );
    assert_eq!(
        call(
            &a,
            "POST",
            "/api/v2/commands",
            json!({"request_id":"reference","command":{"type":"button","button_id":"media"}}),
            true,
            Some(&token),
            "192.168.1.2:5000"
        )
        .await
        .status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        call(
            &a,
            "GET",
            "/socket.io/?EIO=4&transport=polling",
            json!(null),
            false,
            Some("invalid"),
            "localhost:5000"
        )
        .await
        .status(),
        StatusCode::UNAUTHORIZED
    );
}
#[test]
fn invalid_and_cyclic_configured_button_references_are_rejected_before_publication() {
    let mut c: Config =
        serde_json::from_str(include_str!("../webdeck/config_default.json")).unwrap();
    c.layout.folders[0].buttons[0].action = ButtonAction::Command {
        command: Command::Button {
            button_id: "missing".into(),
        },
    };
    assert!(domain::validate_config(&c).is_err());
    c.layout.folders[0].buttons[0].action = ButtonAction::Command {
        command: Command::Button {
            button_id: "media".into(),
        },
    };
    assert!(domain::validate_config(&c).is_err());
}

#[tokio::test]
async fn stalled_action_does_not_block_an_unrelated_action() {
    use std::sync::Barrier;
    struct Stall {
        start: Arc<Barrier>,
        release: Arc<Barrier>,
    }
    impl Adapter for Stall {
        fn execute(&self, c: &Command, _: &Context) -> domain::Result<Value> {
            if matches!(c, Command::PlayPause) {
                self.start.wait();
                self.release.wait();
            }
            Ok(json!({}))
        }
    }
    let start = Arc::new(Barrier::new(2));
    let release = Arc::new(Barrier::new(2));
    let executor = Arc::new(Executor::new(
        Arc::new(Stall {
            start: start.clone(),
            release: release.clone(),
        }),
        2,
    ));
    let owner = executor.clone();
    let stalled = tokio::spawn(async move {
        owner
            .execute(
                CommandRequest {
                    request_id: "stalled".into(),
                    command: Command::PlayPause,
                },
                vec![Capability::Audio],
                || {},
            )
            .await
    });
    tokio::task::spawn_blocking(move || start.wait())
        .await
        .unwrap();
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(1),
        executor.execute(
            CommandRequest {
                request_id: "responsive".into(),
                command: Command::Debug {
                    data: Default::default(),
                },
            },
            vec![Capability::Read],
            || {},
        ),
    )
    .await;
    tokio::task::spawn_blocking(move || release.wait())
        .await
        .unwrap();
    assert!(result.unwrap().is_ok());
    stalled.await.unwrap().unwrap();
}

#[test]
fn grid_dimensions_do_not_limit_button_count() {
    let mut config: Config =
        serde_json::from_str(include_str!("../webdeck/config_default.json")).unwrap();
    config.layout.columns = 1;
    config.layout.rows = 1;
    domain::validate_config(&config).unwrap();
    assert!(config.layout.folders[0].buttons.len() > 1);
}

#[tokio::test(start_paused = true)]
async fn stalled_action_observation_times_out_without_releasing_owned_capacity() {
    use std::sync::Barrier;
    struct Stall {
        start: Arc<Barrier>,
        release: Arc<Barrier>,
    }
    impl Adapter for Stall {
        fn execute(&self, _: &Command, _: &Context) -> domain::Result<Value> {
            self.start.wait();
            self.release.wait();
            Ok(json!({}))
        }
    }
    let start = Arc::new(Barrier::new(2));
    let release = Arc::new(Barrier::new(2));
    let executor = Arc::new(Executor::new(
        Arc::new(Stall {
            start: start.clone(),
            release: release.clone(),
        }),
        1,
    ));
    let owner = executor.clone();
    let observer = tokio::spawn(async move {
        owner
            .execute(
                CommandRequest {
                    request_id: "timeout".into(),
                    command: Command::PlayPause,
                },
                vec![Capability::Audio],
                || {},
            )
            .await
    });
    tokio::task::spawn_blocking(move || start.wait())
        .await
        .unwrap();
    tokio::time::advance(std::time::Duration::from_secs(31)).await;
    let error = observer.await.unwrap().unwrap_err();
    let second = executor
        .execute(
            CommandRequest {
                request_id: "capacity".into(),
                command: Command::PlayPause,
            },
            vec![Capability::Audio],
            || {},
        )
        .await;
    tokio::task::spawn_blocking(move || release.wait())
        .await
        .unwrap();
    assert!(error.message.contains("outcome is unknown"));
    assert_eq!(second.unwrap_err().code, ErrorCode::CapacityExhausted);
    executor.drain().await;
}

#[tokio::test]
async fn paired_settings_credentials_never_become_local_administrators() {
    let t = Temp::new();
    let a = app(&t);
    let grant = a
        .sessions
        .approve(DeviceRequest {
            name: "Settings controller".into(),
            capabilities: vec![Capability::Read, Capability::Settings],
            ttl_seconds: 60,
        })
        .unwrap();
    for remote in [false, true] {
        let host = if remote {
            "192.168.1.2:5000"
        } else {
            "localhost:5000"
        };
        for (method, path, body) in [
            ("GET", "/api/v2/config", json!(null)),
            (
                "PUT",
                "/api/v2/config",
                serde_json::to_value(ConfigRequest {
                    revision: a.config.snapshot().unwrap().revision,
                    config: a.config.snapshot().unwrap().config,
                })
                .unwrap(),
            ),
            ("GET", "/api/v2/devices", json!(null)),
        ] {
            assert_eq!(
                call(&a, method, path, body, remote, Some(&grant.token), host)
                    .await
                    .status(),
                StatusCode::FORBIDDEN
            );
        }
        let response = call(
            &a,
            "GET",
            "/api/v2/boot",
            json!(null),
            remote,
            Some(&grant.token),
            host,
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(response.into_body(), 1024 * 1024)
            .await
            .unwrap();
        let boot: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(boot["can_edit"], false);
        assert!(boot.get("settings").is_none());
    }
}

#[tokio::test]
async fn integration_status_is_local_only_redacted_and_checks_do_not_execute_commands() {
    let t = Temp::new();
    let a = app(&t);
    let snapshot = a.config.snapshot().unwrap();
    a.config
        .mutate(snapshot.revision, |config| {
            config.settings.obs.host = "invalid host".into();
            config.settings.obs.password = "obs-private-secret".into();
            config.settings.spotify.client_id = "private-client-id".into();
            config.settings.spotify.client_secret = "spotify-private-secret".into();
            Ok(())
        })
        .unwrap();
    let grant = a
        .sessions
        .approve(DeviceRequest {
            name: "Controller".into(),
            capabilities: vec![Capability::Read, Capability::Settings],
            ttl_seconds: 60,
        })
        .unwrap();
    for (method, endpoint) in [
        ("GET", "/api/v2/integrations/status"),
        ("POST", "/api/v2/integrations/obs/check"),
    ] {
        assert_eq!(
            call(
                &a,
                method,
                endpoint,
                json!(null),
                false,
                Some(&grant.token),
                "localhost:5000"
            )
            .await
            .status(),
            StatusCode::FORBIDDEN
        );
        let response = call(
            &a,
            method,
            endpoint,
            json!(null),
            false,
            None,
            "localhost:5000",
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(response.into_body(), 4096)
            .await
            .unwrap();
        let status: IntegrationStatus = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(
            status.obs,
            if method == "POST" {
                IntegrationState::Failed
            } else {
                IntegrationState::NotTested
            }
        );
        assert_eq!(status.spotify, IntegrationState::AuthorizationRequired);
        let text = String::from_utf8(bytes.to_vec()).unwrap();
        assert!(!text.contains("private"));
    }
    std::fs::write(t.0.join("spotify-token.json"), b"stored-secret").unwrap();
    let response = call(
        &a,
        "GET",
        "/api/v2/integrations/status",
        json!(null),
        false,
        None,
        "localhost:5000",
    )
    .await;
    let bytes = axum::body::to_bytes(response.into_body(), 4096)
        .await
        .unwrap();
    let status: IntegrationStatus = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(status.spotify, IntegrationState::AuthorizationSaved);
}

#[tokio::test]
async fn disk_capacity_keeps_auth_first_and_direct_effects_independent() {
    let temporary = Temp::new();
    let application = app(&temporary);
    let held = application.io.clone().acquire_many_owned(4).await.unwrap();
    for path in ["/api/v2/boot", "/api/v2/config", "/api/v2/translations"] {
        let response = call(
            &application,
            "GET",
            path,
            json!(null),
            false,
            None,
            "127.0.0.1:5000",
        )
        .await;
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS, "{path}");
    }
    let denied = call(
        &application,
        "GET",
        "/api/v2/boot",
        json!(null),
        false,
        Some("invalid-credential"),
        "127.0.0.1:5000",
    )
    .await;
    assert_eq!(denied.status(), StatusCode::UNAUTHORIZED);
    let direct = call(
        &application,
        "POST",
        "/api/v2/commands",
        json!({"request_id":"independent","command":{"type":"debug","data":{}}}),
        false,
        None,
        "127.0.0.1:5000",
    )
    .await;
    assert_eq!(direct.status(), StatusCode::OK);
    drop(held);
    let restored = call(
        &application,
        "GET",
        "/api/v2/boot",
        json!(null),
        false,
        None,
        "127.0.0.1:5000",
    )
    .await;
    assert_eq!(restored.status(), StatusCode::OK);
}

#[tokio::test]
async fn hardware_query_capacity_does_not_consume_disk_or_effect_capacity() {
    let temporary = Temp::new();
    let application = app(&temporary);
    let held = application
        .queries
        .clone()
        .acquire_many_owned(2)
        .await
        .unwrap();
    for path in ["/api/v2/usage", "/api/v2/audio/devices"] {
        let response = call(
            &application,
            "GET",
            path,
            json!(null),
            false,
            None,
            "127.0.0.1:5000",
        )
        .await;
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS, "{path}");
    }
    let config = call(
        &application,
        "GET",
        "/api/v2/config",
        json!(null),
        false,
        None,
        "127.0.0.1:5000",
    )
    .await;
    assert_eq!(config.status(), StatusCode::OK);
    let direct = call(
        &application,
        "POST",
        "/api/v2/commands",
        json!({"request_id":"independent-query","command":{"type":"debug","data":{}}}),
        false,
        None,
        "127.0.0.1:5000",
    )
    .await;
    assert_eq!(direct.status(), StatusCode::OK);
    drop(held);
}

#[tokio::test]
async fn saturated_authorization_fails_closed_before_effects_and_recovers() {
    let temporary = Temp::new();
    let application = app(&temporary);
    let held = application
        .authorization
        .clone()
        .acquire_many_owned(4)
        .await
        .unwrap();
    for token in [None, Some("invalid-credential")] {
        let response = call(
            &application,
            "POST",
            "/api/v2/commands",
            json!({"request_id":"authorization-full","command":{"type":"debug","data":{}}}),
            false,
            token,
            "127.0.0.1:5000",
        )
        .await;
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    }
    assert_eq!(application.io.available_permits(), 4);
    assert_eq!(application.queries.available_permits(), 2);
    drop(held);
    let denied = call(
        &application,
        "GET",
        "/api/v2/boot",
        json!(null),
        false,
        Some("invalid-credential"),
        "127.0.0.1:5000",
    )
    .await;
    assert_eq!(denied.status(), StatusCode::UNAUTHORIZED);
    let allowed = call(
        &application,
        "POST",
        "/api/v2/commands",
        json!({"request_id":"authorization-restored","command":{"type":"debug","data":{}}}),
        false,
        None,
        "127.0.0.1:5000",
    )
    .await;
    assert_eq!(allowed.status(), StatusCode::OK);
}

#[test]
fn canonical_validation_matches_shared_frontend_corpus() {
    let fixtures: Vec<Value> =
        serde_json::from_str(include_str!("../contracts/validation-cases.json")).unwrap();
    for fixture in fixtures {
        let accepted =
            domain::validate(fixture["definition"].as_str().unwrap(), &fixture["value"]).is_ok();
        assert_eq!(
            accepted,
            fixture["valid"].as_bool().unwrap(),
            "{}",
            fixture["label"]
        );
    }
}
