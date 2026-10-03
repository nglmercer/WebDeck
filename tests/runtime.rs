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
    contracts::{Capability, Command, ErrorCode},
    domain::{self, Result},
    executor::{Adapter, Context, Executor},
    native::Native,
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
fn native(temp: &Temp) -> Arc<Native> {
    Arc::new(
        Native::new(
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
    let temp = Temp::new();
    let native = native(&temp);
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
            native.execute(&command, &context()).unwrap()
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
        capabilities: vec![],
        ..context()
    };
    assert_eq!(
        runtime.invoke(&Command::Usage, &denied).unwrap_err().code,
        ErrorCode::Forbidden
    );
    let expired = Context {
        deadline: Instant::now() - Duration::from_secs(1),
        ..context()
    };
    assert_eq!(
        runtime.invoke(&Command::Usage, &expired).unwrap_err().code,
        ErrorCode::ExecutionFailed
    );
    let deep = Context {
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
        url,
        headers: Default::default(),
        body: String::new(),
        timeout_seconds: 2,
    };
    let network = Context {
        capabilities: vec![Capability::Network],
        ..context()
    };
    let before = native.execute(&command, &network).unwrap();
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
        source: webdeck::contracts::ScriptSource::Inline { code: code.into() },
    };
    let allowed = Context {
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
