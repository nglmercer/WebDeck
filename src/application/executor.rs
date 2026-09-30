use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

use serde_json::Value;
use tokio::sync::{Mutex, Semaphore};

use crate::domain::command::{parse_legacy, Capability, CommandKind, ParsedCommand, Resource};
use crate::domain::error::{AppError, ErrorCode};

/// Ports isolate native effects; contract tests inject deterministic fakes.
pub trait CommandAdapter: Send + Sync + 'static {
    fn execute(&self, command: &ParsedCommand) -> Value;
    fn plugin_names(&self) -> Vec<String> {
        Vec::new()
    }
    fn shutdown(&self) {}
}

pub struct NativeAdapter;
impl CommandAdapter for NativeAdapter {
    fn execute(&self, command: &ParsedCommand) -> Value {
        crate::adapters::platform::execute(command)
    }
    fn plugin_names(&self) -> Vec<String> {
        crate::app::utils::plugins::load_plugins::plugin_commands()
            .values()
            .flat_map(|commands| commands.keys().cloned())
            .collect()
    }
    fn shutdown(&self) {
        crate::adapters::platform::shutdown_children();
    }
}

/// Explicitly safe server mode for automated acceptance runs. It is only
/// selected by the composition root in debug builds, never by a request.
pub struct FakeAdapter;
impl CommandAdapter for FakeAdapter {
    fn execute(&self, _: &ParsedCommand) -> Value {
        serde_json::json!({"success":true})
    }
}

thread_local! {
    static POLICY: RefCell<Option<Vec<Capability>>> = const { RefCell::new(None) };
}

pub fn context_allows(capability: Capability) -> bool {
    POLICY.with(|policy| {
        policy
            .borrow()
            .as_ref()
            .is_none_or(|caps| caps.contains(&capability))
    })
}

struct PolicyGuard(Option<Vec<Capability>>);
impl PolicyGuard {
    fn enter(caps: Option<Vec<Capability>>) -> Self {
        Self(POLICY.with(|policy| policy.replace(caps)))
    }
}
impl Drop for PolicyGuard {
    fn drop(&mut self) {
        POLICY.with(|policy| policy.replace(self.0.take()));
    }
}

pub struct CommandExecutor {
    adapter: Arc<dyn CommandAdapter>,
    admission: Arc<Semaphore>,
    workers: Arc<Semaphore>,
    resources: HashMap<Resource, Arc<Mutex<()>>>,
    capacity: u32,
}

impl CommandExecutor {
    pub fn new(adapter: Arc<dyn CommandAdapter>, capacity: u32, workers: usize) -> Self {
        assert!(capacity > 0 && workers > 0);
        use Resource::*;
        Self {
            adapter,
            admission: Arc::new(Semaphore::new(capacity as usize)),
            workers: Arc::new(Semaphore::new(workers)),
            capacity,
            resources: [
                Read, Input, Audio, Window, Power, Script, Spotify, Obs, Fetch, Plugin, Admin,
            ]
            .into_iter()
            .map(|resource| (resource, Arc::new(Mutex::new(()))))
            .collect(),
        }
    }

    pub fn parse(&self, message: &str) -> Result<ParsedCommand, AppError> {
        parse_legacy(message, &self.adapter.plugin_names())
    }

    /// Admission includes waiting and running work. Owned permits live in the
    /// blocking closure: cancelling a request never frees capacity prematurely.
    /// No request is retried; completed means the adapter returned, not that a
    /// launched external application has finished.
    pub async fn execute(
        &self,
        command: ParsedCommand,
        caps: Option<Vec<Capability>>,
        strict: bool,
    ) -> Result<Value, AppError> {
        self.execute_with_admission(command, caps, strict, || {})
            .await
    }

    /// Reports accepted only after policy and bounded admission succeed.
    /// HTTP waits for completion; realtime callers emit this transition.
    pub async fn execute_with_admission(
        &self,
        command: ParsedCommand,
        caps: Option<Vec<Capability>>,
        strict: bool,
        accepted: impl FnOnce() + Send,
    ) -> Result<Value, AppError> {
        if strict && command.kind == CommandKind::Unknown {
            return Err(AppError::new(ErrorCode::UnknownCommand, "Unknown command"));
        }
        if caps
            .as_ref()
            .is_some_and(|caps| !caps.contains(&command.capability))
        {
            return Err(AppError::new(ErrorCode::Forbidden, "Capability denied"));
        }
        let admission = self.admission.clone().try_acquire_owned().map_err(|e| {
            if e == tokio::sync::TryAcquireError::Closed {
                AppError::new(ErrorCode::ShuttingDown, "Executor is shutting down")
            } else {
                AppError::new(ErrorCode::CapacityExhausted, "Command capacity exhausted")
            }
        })?;
        accepted();
        let resource = self.resources[&command.resource].clone().lock_owned().await;
        let worker = self
            .workers
            .clone()
            .acquire_owned()
            .await
            .map_err(|_| AppError::new(ErrorCode::ShuttingDown, "Executor is shutting down"))?;
        let adapter = self.adapter.clone();
        tokio::task::spawn_blocking(move || {
            let _owned = (admission, resource, worker);
            let _policy = PolicyGuard::enter(caps);
            adapter.execute(&command)
        })
        .await
        .map_err(|_| AppError::new(ErrorCode::ExecutionFailed, "Command execution failed"))
    }

    pub async fn drain(&self) {
        // Mark closed before waiting. A separate lifecycle semaphore cannot be
        // used with close+acquire, so drain observes retained permits instead.
        self.admission.close();
        while self.admission.available_permits() < self.capacity as usize {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        self.adapter.shutdown();
    }
}

pub fn shared() -> Arc<CommandExecutor> {
    static EXECUTOR: OnceLock<Arc<CommandExecutor>> = OnceLock::new();
    EXECUTOR
        .get_or_init(|| {
            let adapter: Arc<dyn CommandAdapter> = if cfg!(debug_assertions)
                && std::env::var("WEBDECK_FAKE_EFFECTS").as_deref() == Ok("1")
            {
                Arc::new(FakeAdapter)
            } else {
                Arc::new(NativeAdapter)
            };
            Arc::new(CommandExecutor::new(adapter, 16, 4))
        })
        .clone()
}
