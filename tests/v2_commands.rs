use serde_json::{json, Value};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use webdeck::{
    application::executor::{context_allows, CommandAdapter, CommandExecutor},
    domain::{
        command::{parse_legacy, Capability, ParsedCommand, BUILTINS},
        error::ErrorCode,
    },
};

#[test]
fn golden_prefix_precedence_aliases_and_raw_arguments() {
    let fixtures: Vec<Value> =
        serde_json::from_str(include_str!("../contracts/legacy-commands.json")).unwrap();
    for fixture in fixtures {
        let raw = fixture["raw"].as_str().unwrap();
        let parsed = parse_legacy(raw, &["custom".into()]).unwrap();
        assert_eq!(
            serde_json::to_value(parsed.kind).unwrap(),
            fixture["kind"],
            "{raw}"
        );
        assert_eq!(
            parsed.arguments,
            fixture["arguments"].as_str().unwrap(),
            "{raw}"
        );
        assert_eq!(parsed.original, raw);
    }
    for descriptor in BUILTINS {
        for prefix in descriptor.prefixes {
            let parsed = parse_legacy(prefix, &[]).unwrap();
            assert_eq!(parsed.kind, descriptor.kind, "{prefix}");
            assert_eq!(parsed.capability, descriptor.capability);
        }
    }
    assert!(parse_legacy(&"x".repeat(65537), &[]).is_err());
    assert!(parse_legacy("a\0b", &[]).is_err());
    // Built-ins outrank overlapping plugin entries.
    assert_eq!(
        parse_legacy("/copy x", &["copy".into()])
            .unwrap()
            .capability,
        Capability::Input
    );
}

struct BlockingFake {
    entered: tokio::sync::Notify,
    release: std::sync::Mutex<std::sync::mpsc::Receiver<()>>,
    calls: AtomicUsize,
}
impl CommandAdapter for BlockingFake {
    fn execute(&self, _: &ParsedCommand) -> Value {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.entered.notify_one();
        self.release.lock().unwrap().recv().unwrap();
        json!({"success":true})
    }
}

#[tokio::test]
async fn overload_and_cancelled_request_retain_running_admission() {
    let (release, receive) = std::sync::mpsc::channel();
    let fake = Arc::new(BlockingFake {
        entered: tokio::sync::Notify::new(),
        release: std::sync::Mutex::new(receive),
        calls: AtomicUsize::new(0),
    });
    let executor = Arc::new(CommandExecutor::new(fake.clone(), 1, 1));
    let work = {
        let executor = executor.clone();
        tokio::spawn(async move {
            executor
                .execute(executor.parse("/key a").unwrap(), None, true)
                .await
        })
    };
    fake.entered.notified().await;
    assert_eq!(
        executor
            .execute(executor.parse("/key b").unwrap(), None, true)
            .await
            .unwrap_err()
            .code,
        ErrorCode::CapacityExhausted
    );
    work.abort();
    assert_eq!(
        executor
            .execute(executor.parse("/key c").unwrap(), None, true)
            .await
            .unwrap_err()
            .code,
        ErrorCode::CapacityExhausted
    );
    release.send(()).unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(2), executor.drain())
        .await
        .unwrap();
    assert_eq!(fake.calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        executor
            .execute(executor.parse("/key d").unwrap(), None, true)
            .await
            .unwrap_err()
            .code,
        ErrorCode::ShuttingDown
    );
}

struct RecordingFake {
    active: AtomicUsize,
    peak: AtomicUsize,
    calls: AtomicUsize,
}
impl CommandAdapter for RecordingFake {
    fn execute(&self, _: &ParsedCommand) -> Value {
        let active = self.active.fetch_add(1, Ordering::SeqCst) + 1;
        self.peak.fetch_max(active, Ordering::SeqCst);
        self.calls.fetch_add(1, Ordering::SeqCst);
        assert!(context_allows(Capability::Input));
        assert!(!context_allows(Capability::Power));
        std::thread::sleep(std::time::Duration::from_millis(15));
        self.active.fetch_sub(1, Ordering::SeqCst);
        json!({"success":true})
    }
}

#[tokio::test]
async fn input_is_ordered_and_denied_unknown_commands_never_reach_effects() {
    let fake = Arc::new(RecordingFake {
        active: AtomicUsize::new(0),
        peak: AtomicUsize::new(0),
        calls: AtomicUsize::new(0),
    });
    let executor = Arc::new(CommandExecutor::new(fake.clone(), 8, 4));
    let mut tasks = Vec::new();
    for _ in 0..4 {
        let executor = executor.clone();
        tasks.push(tokio::spawn(async move {
            executor
                .execute(
                    executor.parse("/key a").unwrap(),
                    Some(vec![Capability::Input]),
                    true,
                )
                .await
        }));
    }
    for task in tasks {
        task.await.unwrap().unwrap();
    }
    assert_eq!(fake.peak.load(Ordering::SeqCst), 1);
    assert_eq!(
        executor
            .execute(
                executor.parse("/PCshutdown").unwrap(),
                Some(vec![Capability::Input]),
                true
            )
            .await
            .unwrap_err()
            .code,
        ErrorCode::Forbidden
    );
    assert_eq!(
        executor
            .execute(executor.parse("/notregistered").unwrap(), None, true)
            .await
            .unwrap_err()
            .code,
        ErrorCode::UnknownCommand
    );
    assert_eq!(fake.calls.load(Ordering::SeqCst), 4);
}
