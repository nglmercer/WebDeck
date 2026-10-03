use axum::{
    body::{to_bytes, Body},
    extract::ConnectInfo,
    http::{Request, StatusCode},
};
use serde_json::{json, Value};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
    thread::ThreadId,
    time::{Duration, Instant},
};
use tower::ServiceExt;
use webdeck::{
    capabilities::Platform,
    contracts::{Capability, Command, ErrorCode},
    domain::{self, Result},
    executor::{Context, Executor},
    runtime::{capabilities::Metrics, VmAdapter, VmRuntime},
    server::{router, App},
    sessions::Sessions,
    storage::{Assets, ConfigStore},
};

#[derive(Default)]
struct FakeMetrics(Mutex<Vec<ThreadId>>);
impl Metrics for FakeMetrics {
    fn usage(&self, context: &Context) -> Result<Value> {
        context.check(Capability::Read)?;
        self.0.lock().unwrap().push(std::thread::current().id());
        Ok(
            json!({"memory_used": 12, "memory_total": 64, "cpu_percent": 25,
            "cpus": [], "disks": [], "gpus": []}),
        )
    }
}
fn context() -> Context {
    Context {
        principal: None,
        owner_id: "test-root".into(),
        capabilities: vec![Capability::Read],
        deadline: Instant::now() + Duration::from_secs(5),
        depth: 0,
    }
}
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("webdeck-vm-{}", domain::id().unwrap()));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn native(temp: &Temp) -> Arc<Platform> {
    Arc::new(
        Platform::new(
            Arc::new(ConfigStore::open(temp.0.join("config.json")).unwrap()),
            Assets {
                root: temp.0.clone(),
            },
            Arc::new(tokio::sync::Notify::new()),
        )
        .unwrap(),
    )
}
#[test]
fn debug_matches_native_for_json_data_and_reuses_vm() {
    let runtime = VmRuntime::new(Arc::new(FakeMetrics::default())).unwrap();
    for data in [
        json!({}),
        json!({"text": "quotes \" and Unicode ñ", "array": [1, false, null]}),
        json!({"code": "__webdeckRuntime = null"}),
    ] {
        let command = Command::Debug {
            data: serde_json::from_value(data).unwrap(),
        };
        assert_eq!(
            runtime.invoke(&command, &context()).unwrap(),
            serde_json::to_value(&command)
                .map(|value| json!({"data":value["data"]}))
                .unwrap()
        );
    }
    runtime.shutdown();
    runtime.shutdown();
    assert!(runtime.invoke(&Command::Usage, &context()).is_err());
}
#[test]
fn metrics_runs_on_one_owner_thread_for_concurrent_callers() {
    let metrics = Arc::new(FakeMetrics::default());
    let runtime = Arc::new(VmRuntime::new(metrics.clone()).unwrap());
    let callers: Vec<_> = (0..8)
        .map(|_| {
            let runtime = runtime.clone();
            std::thread::spawn(move || {
                let result = runtime.invoke(&Command::Usage, &context()).unwrap();
                assert_eq!(result["memory_used"], 12);
            })
        })
        .collect();
    for caller in callers {
        caller.join().unwrap();
    }
    let ids = metrics.0.lock().unwrap();
    assert_eq!(ids.len(), 8);
    assert!(ids
        .iter()
        .all(|id| *id == ids[0] && *id != std::thread::current().id()));
}
#[test]
fn unauthorized_expired_and_deep_requests_do_not_reach_metrics() {
    let metrics = Arc::new(FakeMetrics::default());
    let runtime = VmRuntime::new(metrics.clone()).unwrap();
    let denied = Context {
        principal: None,
        owner_id: "test-root".into(),
        capabilities: vec![],
        ..context()
    };
    assert_eq!(
        runtime.invoke(&Command::Usage, &denied).unwrap_err().code,
        ErrorCode::Forbidden
    );
    let expired = Context {
        principal: None,
        owner_id: "test-root".into(),
        deadline: Instant::now() - Duration::from_secs(1),
        ..context()
    };
    assert_eq!(
        runtime.invoke(&Command::Usage, &expired).unwrap_err().code,
        ErrorCode::ExecutionFailed
    );
    let deep = Context {
        principal: None,
        owner_id: "test-root".into(),
        depth: 9,
        ..context()
    };
    assert!(runtime.invoke(&Command::Usage, &deep).is_err());
    assert!(metrics.0.lock().unwrap().is_empty());
}
#[test]
fn unavailable_commands_fail_without_native_fallback() {
    let runtime = VmRuntime::new(Arc::new(FakeMetrics::default())).unwrap();
    let command = Command::Clipboard;
    let input_context = Context {
        principal: None,
        owner_id: "test-root".into(),
        capabilities: vec![Capability::Input],
        ..context()
    };
    assert_eq!(
        runtime.invoke(&command, &input_context).unwrap_err().code,
        ErrorCode::ExecutionFailed
    );
    assert!(runtime.invoke(&Command::Usage, &context()).is_ok());
}

#[tokio::test]
async fn http_executor_vm_metrics_response_and_drain() {
    let temp = Temp::new();
    let metrics = Arc::new(FakeMetrics::default());
    let runtime = Arc::new(VmRuntime::new(metrics.clone()).unwrap());
    let executor = Arc::new(Executor::new(Arc::new(VmAdapter::new(runtime.clone())), 2));
    let app = App {
        io: Arc::new(tokio::sync::Semaphore::new(4)),
        queries: Arc::new(tokio::sync::Semaphore::new(2)),
        authorization: Arc::new(tokio::sync::Semaphore::new(4)),
        port: 5000,
        plugins: Arc::new(vec![]),
        config: Arc::new(ConfigStore::open(temp.0.join("config.json")).unwrap()),
        sessions: Arc::new(Sessions::open(temp.0.join("devices.json")).unwrap()),
        executor: executor.clone(),
        assets: Assets {
            root: temp.0.clone(),
        },
    };
    let mut catalog_request = Request::builder()
        .uri("/api/v2/commands")
        .header("host", "127.0.0.1:5000")
        .body(Body::empty())
        .unwrap();
    catalog_request.extensions_mut().insert(ConnectInfo(
        "127.0.0.1:35000".parse::<std::net::SocketAddr>().unwrap(),
    ));
    let response = router(app.clone()).oneshot(catalog_request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let catalog: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 1024 * 1024).await.unwrap())
            .unwrap();
    domain::validate("CatalogResponse", &catalog).unwrap();
    assert_eq!(catalog["plugins"].as_array().unwrap().len(), 4);

    for (command, expected) in [
        (
            json!({"type":"debug", "data":{"a":[1,2]}}),
            json!({"data":{"a":[1,2]}}),
        ),
        (
            json!({"type":"usage"}),
            json!({"memory_used":12,"memory_total":64,"cpu_percent":25,"cpus":[],"disks":[],"gpus":[]}),
        ),
    ] {
        let request_id = domain::id().unwrap();
        let mut request = Request::builder()
            .method("POST")
            .uri("/api/v2/commands")
            .header("host", "127.0.0.1:5000")
            .header("content-type", "application/json")
            .body(Body::from(
                json!({"request_id":request_id, "command":command}).to_string(),
            ))
            .unwrap();
        request.extensions_mut().insert(ConnectInfo(
            "127.0.0.1:35000".parse::<std::net::SocketAddr>().unwrap(),
        ));
        let response = router(app.clone()).oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), 65536).await.unwrap()).unwrap();
        assert_eq!(
            body,
            json!({"api_version":2,"request_id":request_id,"state":"completed","result":expected})
        );
    }
    assert_eq!(metrics.0.lock().unwrap().len(), 1);
    executor.drain().await;
    assert!(runtime.invoke(&Command::Usage, &context()).is_err());
}

struct BlockingMetrics {
    entered: std::sync::mpsc::Sender<()>,
    release: Mutex<std::sync::mpsc::Receiver<()>>,
}
impl Metrics for BlockingMetrics {
    fn usage(&self, _: &Context) -> Result<Value> {
        self.entered.send(()).unwrap();
        self.release.lock().unwrap().recv().unwrap();
        Ok(json!({"cpu_percent":25}))
    }
}
#[test]
fn deadline_does_not_release_ownership_while_host_work_is_running() {
    let (entered, started) = std::sync::mpsc::channel();
    let (release, blocked) = std::sync::mpsc::channel();
    let runtime = Arc::new(
        VmRuntime::new(Arc::new(BlockingMetrics {
            entered,
            release: Mutex::new(blocked),
        }))
        .unwrap(),
    );
    let (finished, observed) = std::sync::mpsc::channel();
    let owner = runtime.clone();
    let caller = std::thread::spawn(move || {
        let short = Context {
            principal: None,
            owner_id: "test-root".into(),
            deadline: Instant::now() + Duration::from_millis(200),
            ..context()
        };
        finished
            .send(owner.invoke(&Command::Usage, &short))
            .unwrap();
    });
    started.recv_timeout(Duration::from_secs(5)).unwrap();
    let early = observed.recv_timeout(Duration::from_millis(300));
    release.send(()).unwrap();
    caller.join().unwrap();
    assert!(matches!(
        early,
        Err(std::sync::mpsc::RecvTimeoutError::Timeout)
    ));
    assert_eq!(
        observed.recv().unwrap().unwrap_err().code,
        ErrorCode::ExecutionFailed
    );
    assert!(runtime
        .invoke(
            &Command::Debug {
                data: Default::default()
            },
            &context()
        )
        .is_ok());
}

struct FailingMetrics;
impl Metrics for FailingMetrics {
    fn usage(&self, _: &Context) -> Result<Value> {
        Err(domain::Error::new(
            ErrorCode::Forbidden,
            "Metrics access denied",
        ))
    }
}
#[test]
fn host_errors_keep_v2_codes_and_do_not_poison_later_requests() {
    let runtime = VmRuntime::new(Arc::new(FailingMetrics)).unwrap();
    let error = runtime.invoke(&Command::Usage, &context()).unwrap_err();
    assert_eq!(error.code, ErrorCode::Forbidden);
    assert_eq!(error.message, "Metrics access denied");
    assert_eq!(
        runtime
            .invoke(
                &Command::Debug {
                    data: Default::default()
                },
                &context()
            )
            .unwrap(),
        json!({"data":{}})
    );
}

#[test]
fn fetch_matches_native_against_a_local_server() {
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/parity", listener.local_addr().unwrap());
    let server = std::thread::spawn(move || {
        for _ in 0..2 {
            let (mut stream, _) = listener.accept().unwrap();
            let mut bytes = [0; 4096];
            let length = stream.read(&mut bytes).unwrap();
            assert!(String::from_utf8_lossy(&bytes[..length]).starts_with("GET /parity"));
            stream
                .write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Length: 6\r\nConnection: close\r\n\r\nparity",
                )
                .unwrap();
        }
    });
    let temp = Temp::new();
    let native = native(&temp);
    let runtime = VmRuntime::with_host(Arc::new(FakeMetrics::default()), native.clone()).unwrap();
    let command = Command::Fetch {
        method: "GET".into(),
        url: url.clone(),
        headers: Default::default(),
        body: String::new(),
        timeout_seconds: 2,
    };
    let network = Context {
        principal: None,
        owner_id: "test-root".into(),
        capabilities: vec![Capability::Network],
        ..context()
    };
    use webdeck::runtime::capabilities::CapabilityHost;
    let input = json!({"method":"GET","url":url,"headers":{},"body":"","timeoutMs":2000});
    let response = native.call("network.fetch", &input, &network).unwrap();
    let before = json!({"status":response["status"],"body":response["body"],"truncated":response["truncated"]});
    assert_eq!(runtime.invoke(&command, &network).unwrap(), before);
    server.join().unwrap();
}

#[derive(Default)]
struct RecordingHost(Mutex<Vec<(String, Value)>>);
impl webdeck::runtime::capabilities::CapabilityHost for RecordingHost {
    fn call(&self, operation: &str, input: &Value, context: &Context) -> Result<Value> {
        context.check(webdeck::runtime::capabilities::required_capability(
            operation,
        )?)?;
        self.0
            .lock()
            .unwrap()
            .push((operation.into(), input.clone()));
        Ok(json!({}))
    }
}
#[test]
fn desktop_commands_translate_to_authorized_fake_primitives() {
    let host = Arc::new(RecordingHost::default());
    let runtime = VmRuntime::with_host(Arc::new(FakeMetrics::default()), host.clone()).unwrap();
    let context = Context {
        principal: None,
        owner_id: "test-root".into(),
        capabilities: domain::ALL_CAPABILITIES.to_vec(),
        ..context()
    };
    let cases = [
        (
            json!({"type":"write","text":"hello","send":true}),
            "input.perform",
            json!({"steps":[{"kind":"text","text":"hello"},{"kind":"press","keys":["enter"]}]}),
        ),
        (
            json!({"type":"paste","text":"hello","use_selection":false}),
            "input.perform",
            json!({"steps":[{"kind":"clipboard","text":"hello"},{"kind":"press","keys":["ctrl","v"]}]}),
        ),
        (
            json!({"type":"copy","text":"ignored","use_selection":true}),
            "input.perform",
            json!({"steps":[{"kind":"press","keys":["ctrl","c"]}]}),
        ),
        (
            json!({"type":"cut"}),
            "input.perform",
            json!({"steps":[{"kind":"press","keys":["ctrl","x"]}]}),
        ),
        (
            json!({"type":"open","target":"example.txt"}),
            "window.open",
            json!({"target":"example.txt"}),
        ),
        (
            json!({"type":"close_focused"}),
            "window.closeFocused",
            json!({}),
        ),
        (json!({"type":"color_picker"}), "capture.pick", json!({})),
        (
            json!({"type":"shutdown"}),
            "system.power",
            json!({"verb":"poweroff"}),
        ),
        (
            json!({"type":"reboot"}),
            "system.power",
            json!({"verb":"reboot"}),
        ),
        (
            json!({"type":"lock"}),
            "system.power",
            json!({"verb":"lock"}),
        ),
        (json!({"type":"exit"}), "system.exit", json!({})),
        (
            json!({"type":"mute"}),
            "audio.media",
            json!({"action":"mute"}),
        ),
        (
            json!({"type":"next"}),
            "audio.media",
            json!({"action":"next"}),
        ),
        (
            json!({"type":"volume","change":{"type":"set","percent":25}}),
            "audio.volume",
            json!({"change":{"type":"set","percent":25}}),
        ),
        (
            json!({"type":"microphone","device":"fake"}),
            "audio.endpoint",
            json!({"device":"fake","input":true}),
        ),
        (
            json!({"type":"speakers","device":"fake"}),
            "audio.endpoint",
            json!({"device":"fake","input":false}),
        ),
        (json!({"type":"stop_sound"}), "audio.stopAll", json!({})),
    ];
    for (command, operation, input) in cases {
        let command = serde_json::from_value(command).unwrap();
        runtime.invoke(&command, &context).unwrap();
        assert_eq!(
            host.0.lock().unwrap().pop().unwrap(),
            (operation.to_owned(), input)
        );
    }
    let restart = Command::Restart {
        target: "fake-app".into(),
    };
    runtime.invoke(&restart, &context).unwrap();
    assert_eq!(
        *host.0.lock().unwrap(),
        vec![
            ("window.kill".into(), json!({"target":"fake-app"})),
            (
                "process.spawn".into(),
                json!({"executable":"fake-app","args":[]})
            )
        ]
    );
}

#[test]
fn javascript_scripts_are_isolated_and_nested_calls_keep_the_root_grant() {
    let temp = Temp::new();
    let runtime = VmRuntime::with_host(Arc::new(FakeMetrics::default()), native(&temp)).unwrap();
    let script = |code: &str| Command::Script {
        language: None,
        source: webdeck::contracts::ScriptSource::Inline { code: code.into() },
    };
    let allowed = Context {
        principal: None,
        owner_id: "test-root".into(),
        capabilities: vec![Capability::Script, Capability::Read],
        ..context()
    };
    let result = runtime
        .invoke(
            &script("invoke({type:'debug',data:{text:'nested'}})"),
            &allowed,
        )
        .unwrap();
    assert_eq!(result, json!({"value":{"data":{"text":"nested"}}}));
    assert_eq!(
        runtime
            .invoke(&script("globalThis.leak=42; leak;"), &allowed)
            .unwrap(),
        json!({"value":42})
    );
    assert_eq!(
        runtime.invoke(&script("typeof leak"), &allowed).unwrap(),
        json!({"value":"undefined"})
    );
    let denied = runtime
        .invoke(
            &script("invoke({type:'write',text:'never type',send:false})"),
            &allowed,
        )
        .unwrap_err();
    assert_eq!(denied.code, ErrorCode::Forbidden);
    assert!(runtime.invoke(&script("while(true) {}"), &allowed).is_err());
    assert!(runtime
        .invoke(
            &Command::Debug {
                data: Default::default()
            },
            &allowed
        )
        .is_ok());
}

fn sandbox_plugin(temp: &Temp, id: &str, source: &str, capabilities: Vec<Capability>) {
    use sha2::{Digest, Sha256};
    let root = temp.0.join("plugins").join(id);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("index.js"), source).unwrap();
    let manifest = json!({"schema_version":2,"id":id,"version":"2.0.0","entry":"index.js","backend":"sandbox_js","digest":format!("{:x}",Sha256::digest(source.as_bytes())),"origin":"local","contract":"","actions":[{"id":"echo","label":"Echo","capabilities":capabilities,"arguments":{"text":{"type":"string","required":true}},"result":{"type":"object","required":true}}]});
    std::fs::write(root.join("webdeck.json"), manifest.to_string()).unwrap();
}
#[test]
fn sandbox_plugins_narrow_grants_preserve_state_and_validate_arguments() {
    let temp = Temp::new();
    sandbox_plugin(&temp,"echo","let count=0; export function invoke_action(action,args,ctx) { count++; return ctx.invoke({type:'debug',data:{text:args.text,count}}); }",vec![Capability::Read]);
    sandbox_plugin(&temp,"denied","export function invoke_action(action,args,ctx) { return ctx.invoke({type:'write',text:args.text,send:false}); }",vec![Capability::Read]);
    let plugins = webdeck::runtime::plugins::load_plugins(&Assets {
        root: temp.0.clone(),
    })
    .unwrap();
    let runtime =
        VmRuntime::with_plugins(Arc::new(FakeMetrics::default()), native(&temp), plugins).unwrap();
    let context = Context {
        principal: None,
        owner_id: "test-root".into(),
        capabilities: vec![Capability::Plugin, Capability::Read, Capability::Input],
        ..context()
    };
    let plugin = |id: &str, args| Command::Plugin {
        plugin_id: id.into(),
        version: "2.0.0".into(),
        action_id: "echo".into(),
        args,
    };
    let args = std::collections::BTreeMap::from([("text".into(), json!("hello"))]);
    for count in 1..=2 {
        let result = runtime
            .invoke(&plugin("echo", args.clone()), &context)
            .unwrap();
        assert_eq!(result["value"]["data"]["count"], count);
    }
    assert_eq!(
        runtime
            .invoke(&plugin("denied", args.clone()), &context)
            .unwrap_err()
            .code,
        ErrorCode::Forbidden
    );
    assert!(runtime
        .invoke(&plugin("echo", Default::default()), &context)
        .is_err());
    assert!(runtime
        .invoke(
            &plugin(
                "echo",
                std::collections::BTreeMap::from([("text".into(), json!(123))])
            ),
            &context
        )
        .is_err());
    assert!(runtime.invoke(&plugin("echo", args), &context).is_ok());
    std::fs::write(temp.0.join("plugins/echo/index.js"), "tampered").unwrap();
    assert!(webdeck::runtime::plugins::load_plugins(&Assets {
        root: temp.0.clone()
    })
    .is_err());
}

struct IntegrationHost {
    responses: Mutex<std::collections::VecDeque<Value>>,
    calls: Mutex<Vec<(String, Value)>>,
}
impl webdeck::runtime::capabilities::CapabilityHost for IntegrationHost {
    fn call(&self, operation: &str, input: &Value, context: &Context) -> Result<Value> {
        context.check(webdeck::runtime::capabilities::required_capability(
            operation,
        )?)?;
        self.calls
            .lock()
            .unwrap()
            .push((operation.into(), input.clone()));
        match operation {
            "secrets.integration" => {
                assert!(matches!(
                    context.principal.as_deref(),
                    Some("builtin.obs" | "builtin.spotify")
                ));
                if input["id"] == "obs" {
                    Ok(json!({"host":"127.0.0.1","port":4455,"password":"test-password"}))
                } else {
                    Ok(
                        json!({"clientId":"id","clientSecret":"secret","token":{"access_token":"test-token","expires_at":"2099-01-01T00:00:00Z"}}),
                    )
                }
            }
            "network.wsOpen" => Ok(json!("opaque-connection")),
            "network.wsReceive" | "network.fetch" => self
                .responses
                .lock()
                .unwrap()
                .pop_front()
                .ok_or_else(domain::Error::execution),
            "crypto.sha256Base64" => Ok(json!("test-digest")),
            _ => Ok(Value::Null),
        }
    }
}
#[test]
fn obs_and_spotify_behavior_runs_in_js_over_scoped_fake_io() {
    let host = Arc::new(IntegrationHost {
        responses: Mutex::new(std::collections::VecDeque::from([
            json!({"op":0,"d":{"authentication":{"salt":"salt","challenge":"challenge"}}}),
            json!({"op":2}),
            json!({"op":7,"d":{"requestId":"webdeck","requestStatus":{"result":true}}}),
        ])),
        calls: Mutex::new(vec![]),
    });
    let runtime = VmRuntime::with_host(Arc::new(FakeMetrics::default()), host.clone()).unwrap();
    let context = Context {
        principal: None,
        owner_id: "integration-test".into(),
        capabilities: vec![Capability::Network],
        ..context()
    };
    assert_eq!(
        runtime
            .invoke(
                &Command::Obs {
                    action: "start_recording".into(),
                    target: String::new()
                },
                &context
            )
            .unwrap(),
        json!({})
    );
    assert!(host
        .calls
        .lock()
        .unwrap()
        .iter()
        .any(|(op, input)| op == "network.wsSend"
            && input["value"]["d"]["requestType"] == "StartRecord"));
    host.calls.lock().unwrap().clear();
    *host.responses.lock().unwrap() = std::collections::VecDeque::from([
        json!({"status":200,"body":"{\"device\":{\"volume_percent\":40}}","truncated":false}),
        json!({"status":204,"body":"","truncated":false}),
    ]);
    let command = Command::Spotify {
        action: "volume".into(),
        target: String::new(),
        change: webdeck::contracts::VolumeChange::Adjust { percent: 10 },
    };
    assert_eq!(runtime.invoke(&command, &context).unwrap(), json!({}));
    assert!(host
        .calls
        .lock()
        .unwrap()
        .iter()
        .any(|(op, input)| op == "network.fetch"
            && input["url"] == "https://api.spotify.com/v1/me/player/volume?volume_percent=50"));
}
#[test]
fn integration_secrets_are_not_available_to_general_network_callers() {
    use webdeck::runtime::capabilities::CapabilityHost;
    let temp = Temp::new();
    let platform = native(&temp);
    let context = Context {
        principal: None,
        owner_id: "secret-test".into(),
        capabilities: vec![Capability::Network],
        ..context()
    };
    assert_eq!(
        platform
            .call("secrets.integration", &json!({"id":"obs"}), &context)
            .unwrap_err()
            .code,
        ErrorCode::Forbidden
    );
}

#[tokio::test]
async fn workflows_compose_results_within_one_executor_root() {
    let runtime = Arc::new(VmRuntime::new(Arc::new(FakeMetrics::default())).unwrap());
    let executor = Executor::new(Arc::new(VmAdapter::new(runtime.clone())), 1);
    let command:Command=serde_json::from_value(json!({"type":"workflow","workflow":{"type":"sequence","steps":[
        {"type":"variable","name":"greeting","value":"hello"},
        {"type":"command","command":{"type":"debug","data":{"text":{"$result":"vars.greeting"}}}},
        {"type":"conditional","condition":{"$result":"last.data.text"},"if_true":{"type":"result","path":"last.data.text"},"if_false":{"type":"delay","milliseconds":0}},
        {"type":"parallel","steps":[{"type":"delay","milliseconds":1},{"type":"command","command":{"type":"debug","data":{"parallel":true}}}]}
    ]}})).unwrap();
    let result = executor
        .execute(
            webdeck::contracts::CommandRequest {
                request_id: "workflow-test".into(),
                command,
            },
            vec![Capability::Read],
            || {},
        )
        .await
        .unwrap();
    assert_eq!(
        result,
        json!(["hello",{"data":{"text":"hello"}},"hello",[null,{"data":{"parallel":true}}]])
    );
    executor.drain().await;
}
#[test]
fn workflow_commands_and_references_cannot_escalate_capabilities() {
    let runtime = VmRuntime::new(Arc::new(FakeMetrics::default())).unwrap();
    let command:Command=serde_json::from_value(json!({"type":"workflow","workflow":{"type":"command","command":{"type":"write","text":"never type","send":false}}})).unwrap();
    assert_eq!(
        runtime.invoke(&command, &context()).unwrap_err().code,
        ErrorCode::Forbidden
    );
}

#[test]
fn workflow_async_nodes_settle_and_runtime_remains_reusable() {
    let runtime = VmRuntime::new(Arc::new(FakeMetrics::default())).unwrap();
    for (workflow, expected) in [
        (json!({"type":"variable","name":"a","value":1}), json!(1)),
        (
            json!({"type":"sequence","steps":[{"type":"variable","name":"a","value":1}]}),
            json!([1]),
        ),
        (
            json!({"type":"command","command":{"type":"debug","data":{}}}),
            json!({"data":{}}),
        ),
        (
            json!({"type":"sequence","steps":[{"type":"command","command":{"type":"debug","data":{}}}]}),
            json!([{"data":{}}]),
        ),
        (
            json!({"type":"parallel","steps":[{"type":"delay","milliseconds":1},{"type":"delay","milliseconds":1}]}),
            json!([null, null]),
        ),
    ] {
        let command =
            serde_json::from_value(json!({"type":"workflow","workflow":workflow})).unwrap();
        assert_eq!(runtime.invoke(&command, &context()).unwrap(), expected);
    }
}

#[test]
fn plugin_storage_lifecycle_events_nested_identity_and_reload_are_isolated() {
    let temp = Temp::new();
    sandbox_plugin(
        &temp,
        "inner",
        "export function invoke_action(action,args,ctx) { return {value:args.text}; }",
        vec![Capability::Read],
    );
    sandbox_plugin(&temp, "outer", "import {get,set} from 'webdeck:storage'; let count=0; export function onLoad(ctx){count=10;} export function onUnload(ctx){set('unloaded',true);} export function invoke_action(action,args,ctx) { count++; set('count',count); ctx.emit({api_version:2,type:'button.stateChanged',button_id:'test',label:args.text,active:true}); let inner=ctx.invoke({type:'plugin',plugin_id:'inner',version:'2.0.0',action_id:'echo',args:{text:args.text}}); return {count,stored:get('count'),inner:inner.value}; }", vec![Capability::Read, Capability::Plugin]);
    let assets = Assets {
        root: temp.0.clone(),
    };
    let plugins = webdeck::runtime::plugins::load_plugins(&assets).unwrap();
    let runtime =
        VmRuntime::with_plugins(Arc::new(FakeMetrics::default()), native(&temp), plugins).unwrap();
    let mut events = runtime.events();
    let command: Command = serde_json::from_value(json!({"type":"plugin","plugin_id":"outer","version":"2.0.0","action_id":"echo","args":{"text":"Changed"}})).unwrap();
    let context = Context {
        capabilities: vec![Capability::Read, Capability::Plugin],
        ..context()
    };
    let result = runtime.invoke(&command, &context).unwrap();
    assert_eq!(
        result["value"],
        json!({"count":11,"stored":11,"inner":{"value":"Changed"}})
    );
    assert_eq!(events.try_recv().unwrap()["type"], "button.stateChanged");
    let snapshot = runtime.management(None).unwrap();
    assert_eq!(snapshot["runtime"], "napi-vm");
    assert!(snapshot["loaded_plugins"]
        .as_array()
        .unwrap()
        .contains(&json!("outer")));
    runtime
        .management(Some(
            webdeck::runtime::plugins::load_plugins(&assets).unwrap(),
        ))
        .unwrap();
    assert_eq!(events.try_recv().unwrap()["type"], "runtime.reloaded");
    let state: Value =
        serde_json::from_slice(&std::fs::read(temp.0.join("plugin-state/outer.json")).unwrap())
            .unwrap();
    assert_eq!(state["unloaded"], true);
    assert!(!temp.0.join("plugin-state/inner.json").exists());
    assert_eq!(
        runtime.invoke(&command, &context).unwrap()["value"]["count"],
        11
    );
    // Scripts inherit the caller's permissions but never a plugin identity.
    let script: Command = serde_json::from_value(json!({"type":"script","language":"javascript","source":{"type":"inline","code":"import {get} from 'webdeck:storage'; get('count')"}})).unwrap();
    let context = Context {
        capabilities: vec![Capability::Read, Capability::Script],
        ..context
    };
    assert_eq!(
        runtime.invoke(&script, &context).unwrap_err().code,
        ErrorCode::Forbidden
    );
}

#[test]
fn failing_plugin_is_disabled_until_reload_and_cannot_poison_core_commands() {
    let temp = Temp::new();
    sandbox_plugin(
        &temp,
        "broken",
        "export function invoke_action(){while(true){}}",
        vec![Capability::Read],
    );
    let assets = Assets {
        root: temp.0.clone(),
    };
    let runtime = VmRuntime::with_plugins(
        Arc::new(FakeMetrics::default()),
        native(&temp),
        webdeck::runtime::plugins::load_plugins(&assets).unwrap(),
    )
    .unwrap();
    let command:Command=serde_json::from_value(json!({"type":"plugin","plugin_id":"broken","version":"2.0.0","action_id":"echo","args":{"text":"test"}})).unwrap();
    let context = Context {
        capabilities: vec![Capability::Plugin, Capability::Read],
        ..context()
    };
    assert!(runtime.invoke(&command, &context).is_err());
    assert_eq!(
        runtime.management(None).unwrap()["disabled_plugins"],
        json!(["broken"])
    );
    assert!(runtime.invoke(&Command::Usage, &context).is_ok());
    runtime
        .management(Some(
            webdeck::runtime::plugins::load_plugins(&assets).unwrap(),
        ))
        .unwrap();
    assert_eq!(
        runtime.management(None).unwrap()["disabled_plugins"],
        json!([])
    );
}

#[test]
#[ignore = "requires cargo build --example trusted-plugin and WEBDECK_TRUSTED_FIXTURE"]
fn trusted_process_fixture_verifies_integrity_and_survives_plugin_crash() {
    use sha2::{Digest, Sha256};
    let temp = Temp::new();
    let root = temp.0.join("plugins/fixture");
    std::fs::create_dir_all(&root).unwrap();
    let executable = std::env::var("WEBDECK_TRUSTED_FIXTURE").expect("fixture executable path");
    let entry = if cfg!(windows) {
        "fixture.exe"
    } else {
        "fixture"
    };
    std::fs::copy(executable, root.join(entry)).unwrap();
    let target = serde_json::to_value(napi_vm_plugin_host::Target::current()).unwrap();
    let contract: Value =
        serde_json::from_str(include_str!("../examples/trusted-plugin/contract.json")).unwrap();
    std::fs::write(
        root.join("contract.json"),
        serde_json::to_vec(&contract).unwrap(),
    )
    .unwrap();
    let manifest = json!({"manifestVersion":2,"execution":"trusted-process","id":"fixture","version":"2.0.0","protocol":{"major":1,"minMinor":0,"maxMinor":0},"provides":{"webdeck.fixture":"1.0.0"},"requiresHost":{},"profile":"native-executable","contracts":["contract.json"],"launch":{"kind":"executable","entry":entry,"args":[],"target":target},"assets":[],"dependencies":{"native":[],"services":[],"capabilities":[]}});
    std::fs::write(
        root.join("plugin.json"),
        serde_json::to_vec(&manifest).unwrap(),
    )
    .unwrap();
    let files = std::collections::BTreeMap::from_iter(
        ["plugin.json", "contract.json", entry]
            .into_iter()
            .map(|name| {
                (
                    name,
                    format!(
                        "{:x}",
                        Sha256::digest(std::fs::read(root.join(name)).unwrap())
                    ),
                )
            }),
    );
    let lock = json!({"lockVersion":1,"pluginId":"fixture","pluginVersion":"2.0.0","artifact":{"profile":"native-executable","target":target,"abi":"native-executable"},"interfaces":{"webdeck.fixture":{"version":"1.0.0","digest":contract["digest"]}},"files":files});
    std::fs::write(
        root.join("plugin.lock.json"),
        serde_json::to_vec(&lock).unwrap(),
    )
    .unwrap();
    let package = json!({"schema_version":2,"id":"fixture","version":"2.0.0","entry":"plugin.json","backend":"trusted_process","digest":files["plugin.json"],"origin":"test","contract":"webdeck.fixture","actions":[{"id":"echo","label":"Echo","capabilities":["read"],"arguments":{"text":{"type":"string","required":true}},"result":{"type":"object","required":true}}]});
    std::fs::write(
        root.join("webdeck.json"),
        serde_json::to_vec(&package).unwrap(),
    )
    .unwrap();
    let assets = Assets {
        root: temp.0.clone(),
    };
    let runtime = VmRuntime::with_plugins(
        Arc::new(FakeMetrics::default()),
        native(&temp),
        webdeck::runtime::plugins::load_plugins(&assets).unwrap(),
    )
    .unwrap();
    let command = |text: &str| {
        serde_json::from_value::<Command>(json!({"type":"plugin","plugin_id":"fixture","version":"2.0.0","action_id":"echo","args":{"text":text}})).unwrap()
    };
    let context = Context {
        capabilities: vec![Capability::Read, Capability::Plugin],
        ..context()
    };
    assert_eq!(
        runtime.invoke(&command("hello"), &context).unwrap()["value"],
        json!({"text":"hello"})
    );
    let short = Context {
        deadline: Instant::now() + Duration::from_millis(30),
        ..context.clone()
    };
    let timeout = runtime.invoke(&command("slow"), &short).unwrap_err();
    assert!(timeout.message.contains("outcome is unknown"));
    assert!(runtime.invoke(&Command::Usage, &context).is_ok());
    runtime
        .management(Some(
            webdeck::runtime::plugins::load_plugins(&assets).unwrap(),
        ))
        .unwrap();
    assert!(runtime.invoke(&command("crash"), &context).is_err());
    assert!(runtime.invoke(&Command::Usage, &context).is_ok());
    runtime
        .management(Some(
            webdeck::runtime::plugins::load_plugins(&assets).unwrap(),
        ))
        .unwrap();
    // A changed executable fails the production inventory check before launch.
    std::fs::write(root.join(entry), b"tampered").unwrap();
    assert!(runtime.invoke(&command("hello"), &context).is_err());
}

#[test]
fn effective_registry_matches_contracts_and_external_plugin_controls() {
    let temp = Temp::new();
    sandbox_plugin(
        &temp,
        "echo",
        "export function invoke_action(action,args){return {text:args.text};}",
        vec![Capability::Read],
    );
    let assets = Assets {
        root: temp.0.clone(),
    };
    let runtime = VmRuntime::with_plugins(
        Arc::new(FakeMetrics::default()),
        native(&temp),
        webdeck::runtime::plugins::load_plugins(&assets).unwrap(),
    )
    .unwrap();
    let snapshot = runtime.management(None).unwrap();
    domain::validate("RuntimeSnapshot", &snapshot).unwrap();
    let catalog: Vec<Value> =
        serde_json::from_str(include_str!("../contracts/catalog.json")).unwrap();
    let mut expected = catalog
        .iter()
        .map(|c| c["id"].as_str().unwrap())
        .collect::<Vec<_>>();
    expected.sort();
    let mut actual = snapshot["commands"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["id"].as_str().unwrap())
        .collect::<Vec<_>>();
    actual.sort();
    assert_eq!(actual, expected);
    let disabled = runtime.plugin_enabled("echo", false).unwrap();
    assert_eq!(disabled["disabled_plugins"], json!(["echo"]));
    let command:Command=serde_json::from_value(json!({"type":"plugin","plugin_id":"echo","version":"2.0.0","action_id":"echo","args":{"text":"test"}})).unwrap();
    let context = Context {
        capabilities: vec![Capability::Read, Capability::Plugin],
        ..context()
    };
    assert!(runtime.invoke(&command, &context).is_err());
    assert!(runtime.plugin_enabled("builtin.obs", false).is_err());
    runtime.plugin_enabled("echo", true).unwrap();
    assert_eq!(
        runtime.invoke(&command, &context).unwrap()["value"],
        json!({"text":"test"})
    );
}

#[test]
fn invalid_plugin_packages_are_quarantined_without_rewriting_user_files() {
    let temp = Temp::new();
    sandbox_plugin(
        &temp,
        "valid",
        "export function invoke_action(){return {};}",
        vec![Capability::Read],
    );
    std::fs::write(temp.0.join("plugins/legacy.rhai"), "legacy source").unwrap();
    let assets = Assets {
        root: temp.0.clone(),
    };
    let (plugins, rejected) = webdeck::runtime::plugins::discover_plugins(&assets).unwrap();
    assert_eq!(rejected, 1);
    assert_eq!(plugins.len(), 1);
    assert!(webdeck::runtime::plugins::load_plugins(&assets).is_err());
    assert_eq!(
        std::fs::read_to_string(temp.0.join("plugins/legacy.rhai")).unwrap(),
        "legacy source"
    );
}

#[test]
fn workflow_timeout_and_parallel_references_retain_deadlines_and_prior_results() {
    let runtime = VmRuntime::new(Arc::new(FakeMetrics::default())).unwrap();
    let command:Command=serde_json::from_value(json!({"type":"workflow","workflow":{"type":"sequence","steps":[{"type":"variable","name":"answer","value":42},{"type":"command","command":{"type":"debug","data":{"value":{"$result":"vars.answer"}}}},{"type":"parallel","steps":[{"type":"result","path":"results.1.data.value"},{"type":"result","path":"results.0"}]}]}})).unwrap();
    assert_eq!(
        runtime.invoke(&command, &context()).unwrap(),
        json!([42,{"data":{"value":42}},[42,42]])
    );
    let command:Command=serde_json::from_value(json!({"type":"workflow","workflow":{"type":"timeout","milliseconds":1,"step":{"type":"delay","milliseconds":20}}})).unwrap();
    assert!(runtime.invoke(&command, &context()).is_err());
    assert!(runtime.invoke(&Command::Usage, &context()).is_ok());
}
