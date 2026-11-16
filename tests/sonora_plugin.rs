//! The shipped Sonora plugin, run through the plugin runtime against a fake Sonora socket.
#![cfg(unix)]

use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    io::{BufRead, BufReader, Write},
    os::unix::net::UnixListener,
    path::PathBuf,
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};
use webdeck::{
    capabilities::Platform,
    contracts::{Capability, Command, ErrorCode},
    domain::{self, Result},
    executor::Context,
    runtime::{capabilities::Metrics, VmRuntime},
    storage::{Assets, ConfigStore},
};

const PLUGIN_JS: &str = include_str!("../examples/plugins/sonora/index.js");
const PLUGIN_JSON: &str = include_str!("../examples/plugins/sonora/webdeck.json");

struct NoMetrics;
impl Metrics for NoMetrics {
    fn usage(&self, _: &Context) -> Result<Value> {
        Ok(json!({}))
    }
}

struct Temp(PathBuf);
impl Temp {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "webdeck-sonora-plugin-{name}-{}",
            domain::id().unwrap()
        ));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// A fake Sonora: records every request line and answers with `reply`.
struct FakeSonora {
    socket: PathBuf,
    requests: Arc<Mutex<Vec<String>>>,
}

fn fake_sonora(temp: &Temp, reply: fn(&str) -> String) -> FakeSonora {
    let socket = temp.0.join("sonora.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let requests = Arc::new(Mutex::new(Vec::new()));
    let log = requests.clone();
    thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let mut line = String::new();
            if BufReader::new(&stream).read_line(&mut line).is_err() {
                continue;
            }
            let answer = reply(&line);
            log.lock().unwrap().push(line);
            let _ = (&stream).write_all(answer.as_bytes());
        }
    });
    FakeSonora { socket, requests }
}

/// Sonora's answers for the requests the shipped plugin sends.
fn answer(line: &str) -> String {
    let body = if line.contains("current-track") {
        json!({"value": {"current": null}})
    } else if line.contains("\"like\"") {
        json!({"failed": "no-current-track"})
    } else if line.contains("\"search\"") && !line.contains("\"play\"") {
        json!({"value": {"hits": []}})
    } else {
        json!("done")
    };
    json!({"v": 2, "body": body}).to_string() + "\n"
}

fn wrong_version(_: &str) -> String {
    "{\"v\":3,\"body\":\"done\"}\n".into()
}

fn install_plugin(temp: &Temp) {
    let root = temp.0.join("plugins/sonora");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("index.js"), PLUGIN_JS).unwrap();
    std::fs::write(root.join("webdeck.json"), PLUGIN_JSON).unwrap();
}

fn runtime(temp: &Temp, socket: PathBuf) -> VmRuntime {
    runtime_with(temp, Some(socket))
}

/// `None` means the release socket of this account, as the app uses it.
fn runtime_with(temp: &Temp, socket: Option<PathBuf>) -> VmRuntime {
    install_plugin(temp);
    let plugins = webdeck::runtime::plugins::load_plugins(&Assets {
        root: temp.0.clone(),
    })
    .unwrap();
    let mut platform = Platform::new(
        Arc::new(ConfigStore::open(temp.0.join("config.json")).unwrap()),
        Assets {
            root: temp.0.clone(),
        },
        Arc::new(tokio::sync::Notify::new()),
    )
    .unwrap();
    platform.sonora_socket = socket;
    VmRuntime::with_plugins(Arc::new(NoMetrics), Arc::new(platform), plugins).unwrap()
}

/// A caller holding the plugin grant plus `extra`.
fn grant(extra: &[Capability]) -> Context {
    let mut capabilities = vec![Capability::Plugin];
    capabilities.extend_from_slice(extra);
    Context {
        principal: None,
        owner_id: "test-root".into(),
        capabilities,
        deadline: Instant::now() + Duration::from_secs(10),
        depth: 0,
    }
}

fn act(runtime: &VmRuntime, context: &Context, action: &str, args: Value) -> Result<Value> {
    runtime.invoke(
        &Command::Plugin {
            plugin_id: "sonora".into(),
            version: "1.0.0".into(),
            action_id: action.into(),
            args: serde_json::from_value::<BTreeMap<String, Value>>(args).unwrap(),
        },
        context,
    )
}

fn sent(sonora: &FakeSonora) -> Vec<String> {
    sonora.requests.lock().unwrap().clone()
}

#[test]
fn the_shipped_package_loads_with_its_twenty_actions() {
    let temp = Temp::new("load");
    install_plugin(&temp);
    let plugins = webdeck::runtime::plugins::load_plugins(&Assets {
        root: temp.0.clone(),
    })
    .unwrap();
    assert_eq!(plugins.len(), 1);
    assert_eq!(plugins[0].manifest.id, "sonora");
    assert_eq!(plugins[0].manifest.actions.len(), 20);
}

#[test]
fn reading_actions_return_what_sonora_says() {
    let temp = Temp::new("read");
    let sonora = fake_sonora(&temp, answer);
    let runtime = runtime(&temp, sonora.socket.clone());
    let value = act(&runtime, &grant(&[Capability::Read]), "current", json!({})).unwrap();
    assert_eq!(value["value"], json!({"current": null}));
    assert_eq!(
        sent(&sonora),
        vec!["{\"v\":2,\"body\":{\"playback\":\"current-track\"}}\n".to_string()]
    );
}

#[test]
fn playback_actions_need_the_audio_grant_and_never_reach_sonora() {
    let temp = Temp::new("denied");
    let sonora = fake_sonora(&temp, answer);
    let runtime = runtime(&temp, sonora.socket.clone());
    let error = act(&runtime, &grant(&[Capability::Read]), "next", json!({})).unwrap_err();
    assert_eq!(error.code, ErrorCode::Forbidden);
    assert!(sent(&sonora).is_empty());
}

#[test]
fn volume_is_sent_once_the_audio_grant_is_present() {
    let temp = Temp::new("volume");
    let sonora = fake_sonora(&temp, answer);
    let runtime = runtime(&temp, sonora.socket.clone());
    act(
        &runtime,
        &grant(&[Capability::Audio]),
        "volume",
        json!({"percent": 40}),
    )
    .unwrap();
    assert_eq!(
        sent(&sonora),
        vec!["{\"v\":2,\"body\":{\"playback\":{\"volume\":{\"to\":40}}}}\n".to_string()]
    );
}

#[test]
fn bad_arguments_are_refused_before_anything_is_sent() {
    let temp = Temp::new("args");
    let sonora = fake_sonora(&temp, answer);
    let runtime = runtime(&temp, sonora.socket.clone());
    let context = grant(&[Capability::Audio]);
    assert!(act(&runtime, &context, "seek", json!({"percent": 150})).is_err());
    assert!(act(&runtime, &context, "mute", json!({"state": "loud"})).is_err());
    assert!(act(&runtime, &context, "provider", json!({"slug": ""})).is_err());
    assert!(sent(&sonora).is_empty());
}

#[test]
fn sonora_refusals_come_back_as_readable_errors() {
    let temp = Temp::new("refusal");
    let sonora = fake_sonora(&temp, answer);
    let runtime = runtime(&temp, sonora.socket.clone());
    let error = act(
        &runtime,
        &grant(&[Capability::Audio]),
        "like",
        json!({"state": "on"}),
    )
    .unwrap_err();
    assert!(
        error.message.contains("No track is playing in Sonora"),
        "{}",
        error.message
    );
}

#[test]
fn search_needs_network_and_search_and_play_also_needs_audio() {
    let temp = Temp::new("search");
    let sonora = fake_sonora(&temp, answer);
    let runtime = runtime(&temp, sonora.socket.clone());
    let network = grant(&[Capability::Network]);
    let found = act(
        &runtime,
        &network,
        "search",
        json!({"text": "daft punk", "kind": "song"}),
    )
    .unwrap();
    assert_eq!(found["value"], json!({"hits": []}));
    assert_eq!(
        sent(&sonora),
        vec!["{\"v\":2,\"body\":{\"search\":{\"text\":\"daft punk\",\"kind\":\"song\",\"limit\":5}}}\n"
            .to_string()]
    );

    let denied = act(
        &runtime,
        &network,
        "search_play",
        json!({"text": "daft punk", "kind": "song"}),
    )
    .unwrap_err();
    assert_eq!(denied.code, ErrorCode::Forbidden);
    assert_eq!(sent(&sonora).len(), 1);

    act(
        &runtime,
        &grant(&[Capability::Network, Capability::Audio]),
        "search_play",
        json!({"text": "daft punk", "kind": "song"}),
    )
    .unwrap();
    assert_eq!(
        sent(&sonora)[1],
        "{\"v\":2,\"body\":{\"queue\":{\"play\":[{\"search\":{\"text\":\"daft punk\",\"kind\":\"song\"}}]}}}\n"
    );
}

#[test]
fn a_missing_sonora_is_reported_as_not_running() {
    let temp = Temp::new("missing");
    let runtime = runtime(&temp, temp.0.join("absent.sock"));
    let error = act(&runtime, &grant(&[Capability::Read]), "current", json!({})).unwrap_err();
    assert!(
        error.message.contains("Sonora is not running"),
        "{}",
        error.message
    );
}

#[test]
fn another_protocol_version_is_reported_not_read() {
    let temp = Temp::new("version");
    let sonora = fake_sonora(&temp, wrong_version);
    let runtime = runtime(&temp, sonora.socket.clone());
    let error = act(&runtime, &grant(&[Capability::Read]), "current", json!({})).unwrap_err();
    assert!(error.message.contains("protocol 3"), "{}", error.message);
}

/// Needs a Sonora running on this account, with its socket in `$XDG_RUNTIME_DIR`. Run with:
/// `cargo test --test sonora_plugin -- --ignored live_sonora`
#[test]
#[ignore = "needs a running Sonora on this account"]
fn live_sonora_answers_the_shipped_actions() {
    use webdeck::capabilities::sonora_ipc::{call, default_socket_path, Reply};

    let temp = Temp::new("live");
    let runtime = runtime_with(&temp, None);
    let read = grant(&[Capability::Read]);
    let audio = grant(&[Capability::Audio]);
    let socket = default_socket_path().unwrap();
    let volume_of = || match call(
        &socket,
        &json!({"playback": {"volume": null}}),
        Duration::from_secs(5),
    )
    .unwrap()
    {
        Reply::Value(value) => value["volume"].as_u64().unwrap(),
        other => panic!("unexpected volume reply: {other:?}"),
    };

    let current = act(&runtime, &read, "current", json!({})).unwrap();
    assert!(current["value"].get("current").is_some(), "{current}");

    let queue = act(&runtime, &read, "queue", json!({})).unwrap();
    assert!(queue["value"].get("queue").is_some(), "{queue}");

    let providers = act(&runtime, &read, "providers", json!({})).unwrap();
    let listed = providers["value"]["providers"].as_array().unwrap();
    assert!(
        listed.iter().any(|p| p["current"] == json!(true)),
        "{providers}"
    );

    // Change the volume and put it back, so the instance is left as it was found.
    let original = volume_of();
    let target = if original == 40 { 41 } else { 40 };
    act(&runtime, &audio, "volume", json!({"percent": target})).unwrap();
    assert_eq!(volume_of(), target);
    act(&runtime, &audio, "volume", json!({"percent": original})).unwrap();
    assert_eq!(volume_of(), original);
}

#[test]
fn a_refusal_from_sonora_does_not_disable_the_plugin() {
    let temp = Temp::new("refusal-kept");
    let sonora = fake_sonora(&temp, answer);
    let runtime = runtime(&temp, sonora.socket.clone());
    let context = grant(&[Capability::Audio, Capability::Read]);
    let refused = act(&runtime, &context, "like", json!({"state": "on"})).unwrap_err();
    assert!(
        refused.message.contains("No track is playing"),
        "{}",
        refused.message
    );
    // Only that call failed; the plugin keeps answering.
    let value = act(&runtime, &context, "current", json!({})).unwrap();
    assert_eq!(value["value"], json!({"current": null}));
}

#[test]
fn a_value_outside_the_declared_choices_is_refused_before_sonora() {
    let temp = Temp::new("choices");
    let sonora = fake_sonora(&temp, answer);
    let runtime = runtime(&temp, sonora.socket.clone());
    let error = act(
        &runtime,
        &grant(&[Capability::Network]),
        "search",
        json!({"text": "vaundy", "kind": "vaundy"}),
    )
    .unwrap_err();
    assert_eq!(error.code, ErrorCode::InvalidInput);
    assert!(sent(&sonora).is_empty());
}

#[test]
fn the_shipped_package_declares_the_choices_of_its_arguments() {
    let temp = Temp::new("declared");
    install_plugin(&temp);
    let plugins = webdeck::runtime::plugins::load_plugins(&Assets {
        root: temp.0.clone(),
    })
    .unwrap();
    let search = plugins[0]
        .manifest
        .actions
        .iter()
        .find(|action| action.id == "search")
        .unwrap();
    let kinds = search.arguments["kind"].r#enum.clone().unwrap();
    let kinds: Vec<&str> = kinds.iter().map(String::as_str).collect();
    assert_eq!(kinds, ["song", "album", "artist", "playlist"]);
}
