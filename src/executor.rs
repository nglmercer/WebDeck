use crate::{
    contracts::*,
    domain::{self, Error, Result},
};
use serde_json::Value;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::{Mutex, Semaphore};
#[derive(Clone)]
pub struct Context {
    pub capabilities: Vec<Capability>,
    pub deadline: Instant,
    pub depth: u8,
}
impl Context {
    pub fn check(&self, c: Capability) -> Result<()> {
        if !self.capabilities.contains(&c) {
            return Err(Error::new(ErrorCode::Forbidden, "Capability denied"));
        }
        if self.depth > 8 || Instant::now() > self.deadline {
            return Err(Error::new(
                ErrorCode::ExecutionFailed,
                "Execution budget exhausted",
            ));
        }
        Ok(())
    }
    pub fn nested(&self) -> Self {
        Self {
            depth: self.depth + 1,
            ..self.clone()
        }
    }
}
pub trait Adapter: Send + Sync + 'static {
    fn execute(&self, c: &Command, context: &Context) -> Result<Value>;
    fn shutdown(&self) {}
}
pub struct Executor {
    adapter: Arc<dyn Adapter>,
    admission: Arc<Semaphore>,
    gate: Arc<Mutex<()>>,
    capacity: usize,
}
impl Executor {
    pub fn new(adapter: Arc<dyn Adapter>, capacity: usize) -> Self {
        assert!(capacity > 0);
        Self {
            adapter,
            admission: Arc::new(Semaphore::new(capacity)),
            gate: Arc::new(Mutex::new(())),
            capacity,
        }
    }
    pub async fn execute(
        &self,
        r: CommandRequest,
        caps: Vec<Capability>,
        accepted: impl FnOnce() + Send,
    ) -> Result<Value> {
        domain::validate(
            "CommandRequest",
            &serde_json::to_value(&r).map_err(|_| Error::invalid())?,
        )?;
        domain::validate_command(&r.command)?;
        let context = Context {
            capabilities: caps,
            deadline: Instant::now() + Duration::from_secs(30),
            depth: 0,
        };
        context.check(r.command.capability())?;
        let permit = self.admission.clone().try_acquire_owned().map_err(|e| {
            Error::new(
                if e == tokio::sync::TryAcquireError::Closed {
                    ErrorCode::ShuttingDown
                } else {
                    ErrorCode::CapacityExhausted
                },
                "Executor unavailable",
            )
        })?;
        let adapter = self.adapter.clone();
        let gate = self.gate.clone();
        // Ownership transfers before acceptance is observed. Cancellation drops observation only.
        let task = tokio::spawn(async move {
            let lock = gate.lock_owned().await;
            tokio::task::spawn_blocking(move || {
                let _owned = (permit, lock);
                adapter.execute(&r.command, &context)
            })
            .await
            .map_err(|_| Error::execution())?
        });
        accepted();
        task.await.map_err(|_| Error::execution())?
    }
    pub async fn drain(&self) {
        self.admission.close();
        while self.admission.available_permits() < self.capacity {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        self.adapter.shutdown();
    }
}
pub struct Fake;
impl Adapter for Fake {
    fn execute(&self, c: &Command, x: &Context) -> Result<Value> {
        x.check(c.capability())?;
        Ok(serde_json::json!({"effect":"simulated"}))
    }
}
