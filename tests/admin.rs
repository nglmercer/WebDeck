use serde_json::{json, Value};
use std::io::Write;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use webdeck::admin::{
    actions, buttons, configuration, doctor, folders, ErrorKind, ExitCode, Outcome,
    WebDeckAdminClient,
};
use webdeck::contracts::{Command, ConfigResponse, PluginManifest};
use webdeck::domain::{self, Error as DomainError, Result as DomainResult};
use webdeck::executor::{Adapter, Context, Executor};
use webdeck::runtime::plugins::RuntimePlugin;
use webdeck::server::{router, App};
use webdeck::sessions::Sessions;
use webdeck::storage::{Assets, ConfigStore};

struct Temp {
    path: PathBuf,
}

impl Temp {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("webdeck-admin-{}", domain::id().unwrap()));
        std::fs::create_dir_all(&path).unwrap();
        Self { path }
    }
}

impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

fn echo_manifest() -> PluginManifest {
    serde_json::from_str(include_str!("../examples/plugins/echo/webdeck.json")).unwrap()
}

fn catalog_command_ids() -> Vec<Value> {
    let catalog: Value = serde_json::from_str(include_str!("../contracts/catalog.json")).unwrap();
    catalog
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| json!({"id": entry["id"]}))
        .collect()
}

fn builtin_manifest() -> PluginManifest {
    serde_json::from_value(json!({
        "schema_version": 2,
        "id": "builtin.debug",
        "version": "2.0.0",
        "entry": "index.js",
        "actions": [
            {
                "id": "debug",
                "label": "Debug",
                "capabilities": ["read"],
                "arguments": {},
                "result": {"type": "object", "required": true}
            }
        ],
        "backend": "sandbox_js",
        "digest": format!("{:064x}", 1),
        "origin": "builtin",
        "contract": ""
    }))
    .unwrap()
}

struct FakeAdapter {
    obs_fail: AtomicBool,
    executed: Mutex<Vec<Command>>,
    disabled: Mutex<std::collections::BTreeSet<String>>,
    manifests: Mutex<Vec<PluginManifest>>,
    loaded: Mutex<Vec<String>>,
}

impl FakeAdapter {
    fn new() -> Self {
        Self {
            obs_fail: AtomicBool::new(false),
            executed: Mutex::new(Vec::new()),
            disabled: Mutex::new(std::collections::BTreeSet::new()),
            manifests: Mutex::new(vec![builtin_manifest(), echo_manifest()]),
            loaded: Mutex::new(vec!["echo".to_string()]),
        }
    }
    fn snapshot(&self) -> Value {
        let manifests = self.manifests.lock().unwrap().clone();
        let mut disabled: Vec<String> = self.disabled.lock().unwrap().iter().cloned().collect();
        disabled.sort();
        let loaded = self.loaded.lock().unwrap().clone();
        json!({
            "api_version": 2,
            "runtime": "napi-vm",
            "healthy": true,
            "commands": catalog_command_ids(),
            "plugins": manifests,
            "loaded_plugins": loaded,
            "queue_capacity": 16,
            "disabled_plugins": disabled
        })
    }
}

impl Adapter for FakeAdapter {
    fn execute(&self, command: &Command, _context: &Context) -> DomainResult<Value> {
        self.executed.lock().unwrap().push(command.clone());
        match command {
            Command::Obs { action, .. } => {
                if self.obs_fail.load(Ordering::SeqCst) {
                    return Err(DomainError::execution());
                }
                Ok(match action.as_str() {
                    "get_scenes" => {
                        json!({"scenes": [{"sceneName": "Desk"}, {"sceneName": "BRB"}]})
                    }
                    "get_current_scene" => json!({"currentProgramSceneName": "Desk"}),
                    "get_inputs" => json!({"inputs": [{"inputName": "Mic", "inputMuted": false}]}),
                    "get_hotkeys" => json!({"hotkeys": ["VolumeUp"]}),
                    "get_stream_status" => json!({"outputActive": false}),
                    "get_recording_status" => json!({"outputActive": false}),
                    "get_virtual_camera_status" => json!({"outputActive": false}),
                    "get_version" => json!({"obsVersion": "30.0.0"}),
                    _ => json!({}),
                })
            }
            Command::Debug { data } => Ok(json!({"debug": data})),
            _ => Ok(json!({"ok": true})),
        }
    }
    fn management(&self, plugins: Option<Vec<RuntimePlugin>>) -> DomainResult<Value> {
        if let Some(plugins) = plugins {
            let mut manifests: Vec<PluginManifest> = self
                .manifests
                .lock()
                .unwrap()
                .iter()
                .filter(|m| m.id.starts_with("builtin."))
                .cloned()
                .collect();
            manifests.extend(plugins.into_iter().map(|p| p.manifest));
            manifests.sort_by(|a, b| a.id.cmp(&b.id));
            *self.manifests.lock().unwrap() = manifests;
            self.disabled.lock().unwrap().clear();
        }
        Ok(self.snapshot())
    }
    fn plugin_enabled(&self, id: &str, enabled: bool) -> DomainResult<Value> {
        if id.starts_with("builtin.") || !self.manifests.lock().unwrap().iter().any(|m| m.id == id)
        {
            return Err(DomainError::invalid());
        }
        let mut disabled = self.disabled.lock().unwrap();
        if enabled {
            disabled.remove(id);
        } else {
            disabled.insert(id.to_string());
        }
        drop(disabled);
        Ok(self.snapshot())
    }
}

struct TestServer {
    url: String,
    config_dir: PathBuf,
    store: Arc<ConfigStore>,
    adapter: Arc<FakeAdapter>,
    _temp: Temp,
}

fn spawn() -> TestServer {
    let temp = Temp::new();
    let config_dir = temp.path.clone();
    let store = Arc::new(ConfigStore::open(config_dir.join("config.json")).unwrap());
    let adapter = Arc::new(FakeAdapter::new());
    let (ready, received) = std::sync::mpsc::channel();
    let dir = config_dir.clone();
    let server_store = store.clone();
    let server_adapter = adapter.clone();
    std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async move {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let port = listener.local_addr().unwrap().port();
            let app = App {
                io: Arc::new(tokio::sync::Semaphore::new(4)),
                queries: Arc::new(tokio::sync::Semaphore::new(2)),
                authorization: Arc::new(tokio::sync::Semaphore::new(4)),
                port,
                plugins: Arc::new(vec![builtin_manifest(), echo_manifest()]),
                config: server_store,
                sessions: Arc::new(Sessions::open(dir.join("devices.v2.json")).unwrap()),
                executor: Arc::new(Executor::new(server_adapter, 16)),
                assets: Assets { root: dir.clone() },
            };
            let make = router(app).into_make_service_with_connect_info::<SocketAddr>();
            tokio::spawn(async move {
                axum::serve(listener, make).await.unwrap();
            });
            ready.send(format!("http://127.0.0.1:{port}")).unwrap();
            std::future::pending::<()>().await;
        });
    });
    let url = received.recv().unwrap();
    TestServer {
        url,
        config_dir,
        store,
        adapter,
        _temp: temp,
    }
}

fn client(url: &str) -> WebDeckAdminClient {
    WebDeckAdminClient::new(url)
}

fn block_on_test<F: std::future::Future>(future: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(future)
}

fn write_file(dir: &Path, name: &str, value: &Value) -> String {
    let path = dir.join(name);
    std::fs::write(&path, serde_json::to_vec_pretty(value).unwrap()).unwrap();
    path.to_string_lossy().into_owned()
}

fn find_button(config: &webdeck::contracts::Config, folder: &str, id: &str) -> Value {
    let button = config
        .layout
        .folders
        .iter()
        .find(|f| f.id == folder)
        .and_then(|f| f.buttons.iter().find(|b| b.id == id))
        .unwrap();
    serde_json::to_value(button).unwrap()
}

async fn buttons_ensure(client: &WebDeckAdminClient, folder: &str, value: Value) -> Outcome {
    let temp = Temp::new();
    let path = write_file(&temp.path, "button.json", &value);
    buttons::ensure(client, folder, Some(&path), None, None, false)
        .await
        .unwrap()
}

#[tokio::test]
async fn status_reports_revision_runtime_and_integrations() {
    let server = spawn();
    let client = client(&server.url);
    let status = doctor::status(&client).await.unwrap();
    assert!(status.ok);
    assert_eq!(status.data["api_version"], 2);
    assert_eq!(status.data["healthy"], true);
    assert_eq!(status.data["can_edit"], true);
    assert!(status.data["revision"].as_u64().unwrap() >= 1);
    assert!(status.data["server_version"].as_str().is_some());
    assert!(status.human.contains("WebDeck"));
    let version = client.version().await.unwrap();
    assert_eq!(version.api_version, 2);
}

#[tokio::test]
async fn doctor_reports_passing_checks() {
    let server = spawn();
    let client = client(&server.url);
    let outcome = doctor::doctor(&client).await.unwrap();
    assert!(outcome.ok, "{}", outcome.human);
    assert_eq!(outcome.exit, 0);
    assert_eq!(outcome.data["failures"], 0);
    let checks = outcome.data["checks"].as_array().unwrap();
    assert!(checks.iter().any(|c| c["id"] == "server.reachable"));
    assert!(checks.iter().any(|c| c["id"] == "config.valid"));
}

#[tokio::test]
async fn config_get_redacts_secrets_unless_revealed() {
    let server = spawn();
    let revision = server.store.snapshot().unwrap().revision;
    server
        .store
        .mutate(revision, |config| {
            config.settings.obs.password = "s3cret".into();
            config.settings.spotify.client_secret = "top".into();
            Ok(())
        })
        .unwrap();
    let client = client(&server.url);
    let outcome = configuration::get(&client, false).await.unwrap();
    assert_eq!(
        outcome.data["config"]["settings"]["obs"]["password"],
        "password_configured"
    );
    assert_eq!(
        outcome.data["config"]["settings"]["spotify"]["client_secret"],
        true
    );
    assert!(!outcome.human.contains("s3cret"));
    let outcome = configuration::get(&client, true).await.unwrap();
    assert_eq!(
        outcome.data["config"]["settings"]["obs"]["password"],
        "s3cret"
    );
    assert_eq!(outcome.data["redacted"], false);
}

#[tokio::test]
async fn config_validate_flags_invalid_documents() {
    let server = spawn();
    let client = client(&server.url);
    let outcome = configuration::validate(&client, None).await.unwrap();
    assert!(outcome.ok);
    assert_eq!(outcome.data["valid"], true);
    let path = write_file(
        &server.config_dir,
        "bad.json",
        &json!({"schema_version": 1, "settings": {}, "layout": {}, "extensions": {}}),
    );
    let outcome = configuration::validate(&client, Some(&path)).await.unwrap();
    assert!(!outcome.ok);
    assert_eq!(outcome.exit, ExitCode::Validation as i32);
    assert_eq!(outcome.data["valid"], false);
    assert!(!outcome.data["errors"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn folder_ensure_is_idempotent_and_delete_checks_existence() {
    let server = spawn();
    let client = client(&server.url);
    let created = folders::ensure(&client, None, Some("ops"), Some("Ops"), None, false)
        .await
        .unwrap();
    assert_eq!(created.data["applied"], true);
    assert_eq!(created.data["operation"], "folder_ensure");
    let again = folders::ensure(&client, None, Some("ops"), None, None, false)
        .await
        .unwrap();
    assert_eq!(again.data["applied"], false);
    let relabel = folders::ensure(&client, None, Some("ops"), Some("Operations"), None, false)
        .await
        .unwrap();
    assert_eq!(relabel.data["applied"], true);
    let response = client.get_config().await.unwrap();
    let folder = response
        .config
        .layout
        .folders
        .iter()
        .find(|f| f.id == "ops")
        .unwrap();
    assert_eq!(folder.label, "Operations");
    let error = folders::delete(&client, "ghost", None, false)
        .await
        .unwrap_err();
    assert_eq!(error.kind(), ErrorKind::NotFound);
    assert_eq!(error.kind().exit(), ExitCode::NotFound);
    folders::delete(&client, "ops", None, false).await.unwrap();
}

#[tokio::test]
async fn folder_create_conflicts_and_reports_dry_runs() {
    let server = spawn();
    let client = client(&server.url);
    let dry = folders::create(&client, None, Some("ops"), None, None, true)
        .await
        .unwrap();
    assert_eq!(dry.data["applied"], false);
    assert_eq!(dry.data["dry_run"], true);
    let response = client.get_config().await.unwrap();
    assert!(response.config.layout.folders.iter().all(|f| f.id != "ops"));
    folders::create(&client, None, Some("ops"), None, None, false)
        .await
        .unwrap();
    let error = folders::create(&client, None, Some("ops"), None, None, false)
        .await
        .unwrap_err();
    assert_eq!(error.kind(), ErrorKind::RevisionConflict);
    assert_eq!(error.kind().exit(), ExitCode::Conflict);
}

#[tokio::test]
async fn button_ensure_preserves_unspecified_fields() {
    let server = spawn();
    let client = client(&server.url);
    let created = buttons_ensure(&client, "home", json!({"id": "task1", "label": "Task"})).await;
    assert_eq!(created.data["applied"], true);
    let response = client.get_config().await.unwrap();
    let button = find_button(&response.config, "home", "task1");
    assert_eq!(button["icon"], "");
    assert_eq!(button["action"]["type"], "none");
    let updated = buttons_ensure(&client, "home", json!({"id": "task1", "label": "Renamed"})).await;
    assert_eq!(updated.data["applied"], true);
    let response = client.get_config().await.unwrap();
    let button = find_button(&response.config, "home", "task1");
    assert_eq!(button["label"], "Renamed");
    assert_eq!(button["icon"], "");
    let unchanged =
        buttons_ensure(&client, "home", json!({"id": "task1", "label": "Renamed"})).await;
    assert_eq!(unchanged.data["applied"], false);
}

#[tokio::test]
async fn config_apply_reports_changes_and_is_idempotent() {
    let server = spawn();
    let client = client(&server.url);
    let current = client.get_config().await.unwrap();
    let mut target = current.config.clone();
    target.layout.columns = 7;
    target.settings.language = "pt_BR".into();
    let path = write_file(
        &server.config_dir,
        "target.json",
        &serde_json::to_value(&target).unwrap(),
    );
    let diff = configuration::diff(&client, Some(&path)).await.unwrap();
    let changes = diff.data["changes"].as_array().unwrap();
    assert_eq!(diff.data["change_count"], 2);
    assert!(changes.iter().any(|c| c["operation"] == "update_layout"));
    assert!(changes.iter().any(|c| c["operation"] == "update_settings"));
    let dry = configuration::apply(&client, Some(&path), None, true)
        .await
        .unwrap();
    assert_eq!(dry.data["dry_run"], true);
    assert_eq!(dry.data["applied"], false);
    let applied = configuration::apply(&client, Some(&path), None, false)
        .await
        .unwrap();
    assert_eq!(applied.data["applied"], true);
    assert_ne!(applied.data["revision"].as_u64().unwrap(), current.revision);
    let again = configuration::apply(&client, Some(&path), None, false)
        .await
        .unwrap();
    assert_eq!(again.data["applied"], false);
    assert_eq!(again.data["change_count"], 0);
}

#[tokio::test]
async fn stale_revisions_return_conflicts_with_expected_and_actual() {
    let server = spawn();
    let client = client(&server.url);
    let current = client.get_config().await.unwrap();
    let path = write_file(
        &server.config_dir,
        "stale.json",
        &serde_json::to_value(&current.config).unwrap(),
    );
    let error = configuration::apply(&client, Some(&path), Some(current.revision + 5), false)
        .await
        .unwrap_err();
    assert_eq!(error.kind(), ErrorKind::RevisionConflict);
    let details = error.details().unwrap();
    assert_eq!(details["expected"], current.revision + 5);
    assert_eq!(details["actual"], current.revision);
    let error = folders::delete(&client, "home", Some(current.revision + 5), false)
        .await
        .unwrap_err();
    assert_eq!(error.kind(), ErrorKind::RevisionConflict);
    assert_eq!(error.kind().exit(), ExitCode::Conflict);
}

#[tokio::test]
async fn action_run_executes_commands_and_maps_obs_failures() {
    let server = spawn();
    let client = client(&server.url);
    let outcome = actions::run(
        &client,
        Some("debug"),
        None,
        Some(r#"{"data": {"text": "hi"}}"#),
        &[],
        false,
    )
    .await
    .unwrap();
    assert_eq!(outcome.data["state"], "completed");
    assert_eq!(outcome.data["result"]["debug"]["text"], "hi");
    assert!(server
        .adapter
        .executed
        .lock()
        .unwrap()
        .iter()
        .any(|c| matches!(c, Command::Debug { .. })));
    let scenes = actions::run(
        &client,
        Some("obs"),
        None,
        Some(r#"{"action": "get_scenes", "target": ""}"#),
        &[],
        false,
    )
    .await
    .unwrap();
    assert_eq!(scenes.data["result"]["scenes"][0]["sceneName"], "Desk");
    server.adapter.obs_fail.store(true, Ordering::SeqCst);
    let error = actions::run(
        &client,
        Some("obs"),
        None,
        Some(r#"{"action": "get_scenes", "target": ""}"#),
        &[],
        false,
    )
    .await
    .unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Integration);
    assert_eq!(error.kind().exit(), ExitCode::Integration);
    server.adapter.obs_fail.store(false, Ordering::SeqCst);
}

#[tokio::test]
async fn action_dry_run_does_not_execute() {
    let server = spawn();
    let client = client(&server.url);
    let outcome = actions::run(
        &client,
        Some("debug"),
        None,
        Some(r#"{"data": {"text": "hi"}}"#),
        &[],
        true,
    )
    .await
    .unwrap();
    assert_eq!(outcome.data["dry_run"], true);
    assert!(server.adapter.executed.lock().unwrap().is_empty());
}

#[tokio::test]
async fn action_describe_and_list_expose_catalog() {
    let server = spawn();
    let client = client(&server.url);
    let list = actions::list(&client).await.unwrap();
    let ids: Vec<&str> = list.data["actions"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|a| a["id"].as_str())
        .collect();
    assert!(ids.contains(&"obs"));
    let describe = actions::describe(&client, "obs").await.unwrap();
    assert_eq!(describe.data["id"], "obs");
    assert!(describe.data["schema"].is_object());
    let error = actions::describe(&client, "nope").await.unwrap_err();
    assert_eq!(error.kind(), ErrorKind::NotFound);
}

#[tokio::test]
async fn obs_status_check_and_queries_work() {
    let server = spawn();
    let client = client(&server.url);
    let obs = client.obs();
    let status = obs.status().await.unwrap();
    assert!(status.ok);
    let check = obs.check().await.unwrap();
    assert!(check.ok);
    assert_eq!(check.data["obs"], "connected");
    let scenes = obs.scenes().await.unwrap();
    assert!(scenes.human.contains("Desk"));
    let current = obs.current_scene().await.unwrap();
    assert_eq!(current.human, "Desk");
    let inputs = obs.inputs().await.unwrap();
    assert!(inputs.human.contains("Mic"));
    let actions_out = obs.actions().await.unwrap();
    let actions_list = actions_out.data["actions"].as_array().unwrap();
    assert!(actions_list.iter().any(|a| a == "get_scenes"));
    server.adapter.obs_fail.store(true, Ordering::SeqCst);
    let check = obs.check().await.unwrap();
    assert!(!check.ok);
    assert_eq!(check.exit, ExitCode::Integration as i32);
    server.adapter.obs_fail.store(false, Ordering::SeqCst);
}

#[tokio::test]
async fn obs_configure_updates_settings() {
    let server = spawn();
    let client = client(&server.url);
    let obs = client.obs();
    let outcome = obs
        .configure(Some("192.168.1.10"), Some(4456), false, None, false)
        .await
        .unwrap();
    assert_eq!(outcome.data["applied"], true);
    assert_eq!(outcome.data["obs"]["host"], "192.168.1.10");
    assert_eq!(outcome.data["obs"]["password_set"], false);
    let response = client.get_config().await.unwrap();
    assert_eq!(response.config.settings.obs.host, "192.168.1.10");
    assert_eq!(response.config.settings.obs.port, 4456);
    let error = obs
        .configure(None, None, false, None, false)
        .await
        .unwrap_err();
    assert_eq!(error.kind(), ErrorKind::InvalidArguments);
    let error = obs
        .configure(Some("x"), Some(70000), false, None, false)
        .await
        .unwrap_err();
    assert_eq!(error.kind(), ErrorKind::InvalidArguments);
}

#[tokio::test]
async fn plugin_enable_disable_roundtrip_and_builtin_guard() {
    let server = spawn();
    let client = client(&server.url);
    let service = client.plugins(server.config_dir.clone());
    let outcome = service.set_enabled("echo", false).await.unwrap();
    assert_eq!(outcome.data["applied"], true);
    assert_eq!(outcome.data["enabled"], false);
    let list = service.list().await.unwrap();
    let echo = list.data["plugins"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == "echo")
        .unwrap()
        .clone();
    assert_eq!(echo["enabled"], false);
    let outcome = service.set_enabled("echo", true).await.unwrap();
    assert_eq!(outcome.data["applied"], true);
    let error = service
        .set_enabled("builtin.debug", false)
        .await
        .unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Plugin);
    assert_eq!(error.kind().exit(), ExitCode::Plugin);
    let error = service.set_enabled("ghost", true).await.unwrap_err();
    assert_eq!(error.kind(), ErrorKind::NotFound);
}

#[tokio::test]
async fn plugin_init_scaffold_validates() {
    let server = spawn();
    let client = client(&server.url);
    let service = client.plugins(server.config_dir.clone());
    let target = server.config_dir.join("sample");
    let outcome = service
        .init("sample", Some(target.clone()), "2.0.0")
        .unwrap();
    assert_eq!(outcome.data["scaffolded"], true);
    assert!(target.join("index.js").exists());
    assert!(target.join("webdeck.json").exists());
    let validated = service.validate_package(target.to_str().unwrap()).unwrap();
    assert_eq!(validated.data["valid"], true);
    assert_eq!(validated.data["id"], "sample");
    let error = service.validate_package("ghost-plugin").unwrap_err();
    assert_eq!(error.kind(), ErrorKind::NotFound);
    let error = service.init("bad id", None, "2.0.0").unwrap_err();
    assert_eq!(error.kind(), ErrorKind::InvalidArguments);
}

#[tokio::test]
async fn plugin_install_update_uninstall_roundtrip() {
    let server = spawn();
    let client = client(&server.url);
    let service = client.plugins(server.config_dir.clone());
    let first = server.config_dir.join("pkg1").join("echo");
    service.init("echo", Some(first.clone()), "2.0.0").unwrap();
    let installed = service.install(first.to_str().unwrap()).await.unwrap();
    assert_eq!(installed.data["installed"], true);
    assert!(server.config_dir.join("plugins").join("echo").exists());
    let installed_again = service.install(first.to_str().unwrap()).await.unwrap_err();
    assert_eq!(installed_again.kind(), ErrorKind::Plugin);
    let inspected = service.inspect("echo").await.unwrap();
    assert_eq!(inspected.data["version"], "2.0.0");
    let second = server.config_dir.join("pkg2").join("echo");
    service.init("echo", Some(second.clone()), "2.1.0").unwrap();
    let updated = service.update(second.to_str().unwrap()).await.unwrap();
    assert_eq!(updated.data["updated"], true);
    let inspected = service.inspect("echo").await.unwrap();
    assert_eq!(inspected.data["version"], "2.1.0");
    service.uninstall("echo").await.unwrap();
    let error = service.inspect("echo").await.unwrap_err();
    assert_eq!(error.kind(), ErrorKind::NotFound);
    let error = service.uninstall("echo").await.unwrap_err();
    assert_eq!(error.kind(), ErrorKind::NotFound);
    let error = service.uninstall("builtin.debug").await.unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Plugin);
}

struct CliOutput {
    code: i32,
    stdout: String,
    stderr: String,
}

fn run_cli_with_stdin(args: &[&str], home: &Path, input: Option<&str>) -> CliOutput {
    use std::process::Stdio;
    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_webdeckctl"))
        .args(args)
        .env("HOME", home)
        .env("USERPROFILE", home)
        .env_remove("WEBDECK_URL")
        .env_remove("WEBDECK_ADMIN_TOKEN")
        .env_remove("WEBDECK_DEVICE_TOKEN")
        .env_remove("WEBDECK_CONFIG_DIR")
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    if let Some(input) = input {
        child
            .stdin
            .take()
            .unwrap()
            .write_all(input.as_bytes())
            .unwrap();
    }
    let output = child.wait_with_output().unwrap();
    CliOutput {
        code: output.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    }
}

fn run_cli(args: &[&str], home: &Path) -> CliOutput {
    run_cli_with_stdin(args, home, None)
}

#[test]
fn cli_json_envelope_is_pure_and_exit_codes_match() {
    let server = spawn();
    let home = Temp::new();
    let output = run_cli(&["status", "--url", &server.url, "--json"], &home.path);
    assert_eq!(output.code, 0, "{}", output.stderr);
    assert_eq!(output.stdout.trim().lines().count(), 1);
    let value: Value = serde_json::from_str(output.stdout.trim()).unwrap();
    assert_eq!(value["ok"], true);
    assert_eq!(value["data"]["api_version"], 2);
    assert!(value["error"].is_null());

    let output = run_cli(
        &["folder", "delete", "ghost", "--url", &server.url, "--json"],
        &home.path,
    );
    assert_eq!(output.code, ExitCode::NotFound as i32);
    let value: Value = serde_json::from_str(output.stdout.trim()).unwrap();
    assert_eq!(value["ok"], false);
    assert_eq!(value["error"]["code"], "not_found");

    let output = run_cli(&["folder", "--url", &server.url], &home.path);
    assert_eq!(output.code, 2);

    let output = run_cli(
        &["config", "get", "--url", "http://127.0.0.1:1", "--json"],
        &home.path,
    );
    assert_eq!(output.code, ExitCode::Connection as i32);
    let value: Value = serde_json::from_str(output.stdout.trim()).unwrap();
    assert_eq!(value["error"]["code"], "connection_failed");
}

#[test]
fn cli_human_mode_prints_readable_status() {
    let server = spawn();
    let home = Temp::new();
    let output = run_cli(&["status", "--url", &server.url], &home.path);
    assert_eq!(output.code, 0, "{}", output.stderr);
    assert!(output.stdout.contains("WebDeck"));
    assert!(output.stdout.contains("revision"));
}

#[test]
fn cli_config_apply_via_subprocess_roundtrip() {
    let server = spawn();
    let home = Temp::new();
    let config: ConfigResponse = block_on_test(client(&server.url).get_config()).unwrap();
    let mut target = config.config;
    target.layout.rows = 5;
    let path = write_file(
        &server.config_dir,
        "cli-apply.json",
        &serde_json::to_value(&target).unwrap(),
    );
    let output = run_cli(
        &[
            "apply",
            "--file",
            &path,
            "--url",
            &server.url,
            "--json",
            "--dry-run",
        ],
        &home.path,
    );
    assert_eq!(output.code, 0, "{}", output.stderr);
    let value: Value = serde_json::from_str(output.stdout.trim()).unwrap();
    assert_eq!(value["ok"], true);
    assert_eq!(value["data"]["dry_run"], true);
    let output = run_cli(
        &["apply", "--file", &path, "--url", &server.url, "--json"],
        &home.path,
    );
    assert_eq!(output.code, 0, "{}", output.stderr);
    let value: Value = serde_json::from_str(output.stdout.trim()).unwrap();
    assert_eq!(value["data"]["applied"], true);
    let response = block_on_test(client(&server.url).get_config()).unwrap();
    assert_eq!(response.config.layout.rows, 5);
}

#[test]
fn cli_obs_configure_reads_password_from_stdin_without_leaking_it() {
    let server = spawn();
    let home = Temp::new();
    let output = run_cli_with_stdin(
        &[
            "obs",
            "configure",
            "--password-stdin",
            "--url",
            &server.url,
            "--json",
        ],
        &home.path,
        Some("hunter2\n"),
    );
    assert_eq!(output.code, 0, "{}", output.stderr);
    assert!(!output.stdout.contains("hunter2"));
    assert!(!output.stderr.contains("hunter2"));
    let value: Value = serde_json::from_str(output.stdout.trim()).unwrap();
    assert_eq!(value["ok"], true);
    assert_eq!(value["data"]["obs"]["password_set"], true);
    let response = block_on_test(client(&server.url).get_config()).unwrap();
    assert_eq!(response.config.settings.obs.password, "hunter2");
}

#[cfg(feature = "integration-tests")]
#[tokio::test]
async fn live_server_status() {
    let url = std::env::var("WEBDECK_URL").expect("WEBDECK_URL must point at a running server");
    let client = client(&url);
    doctor::status(&client).await.expect("live status");
}
