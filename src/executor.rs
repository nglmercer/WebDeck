mod diagnostics;
use crate::{
    contracts::*,
    domain::{self, Error, Result},
};
use diagnostics::Diagnostic;
use serde_json::Value;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::Semaphore;
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
    pub fn remaining(&self, capability: Capability, ceiling: Duration) -> Result<Duration> {
        self.check(capability)?;
        let remaining = self.deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(Error::new(
                ErrorCode::ExecutionFailed,
                "Execution budget exhausted",
            ));
        }
        Ok(remaining.min(ceiling))
    }
}
pub trait Adapter: Send + Sync + 'static {
    fn execute(&self, c: &Command, context: &Context) -> Result<Value>;
    fn shutdown(&self) {}
}
pub struct Executor {
    adapter: Arc<dyn Adapter>,
    admission: Arc<Semaphore>,
    capacity: usize,
}
impl Executor {
    pub fn new(adapter: Arc<dyn Adapter>, capacity: usize) -> Self {
        assert!(capacity > 0);
        Self {
            adapter,
            admission: Arc::new(Semaphore::new(capacity)),
            capacity,
        }
    }
    pub async fn execute(
        &self,
        r: CommandRequest,
        caps: Vec<Capability>,
        accepted: impl FnOnce() + Send,
    ) -> Result<Value> {
        let mut diagnostic = Diagnostic::new(&r.request_id, r.command.capability());
        let preparation: Result<_> = (|| {
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
            Ok((context, permit))
        })();
        let (context, permit) = match preparation {
            Ok(ready) => ready,
            Err(error) => {
                diagnostic.finish("rejected", Some(error.code));
                return Err(error);
            }
        };
        let adapter = self.adapter.clone();
        // Ownership transfers before acceptance is observed. Cancellation drops observation only.
        let task = tokio::spawn(async move {
            tokio::task::spawn_blocking(move || {
                let _owned = permit;
                let result = context
                    .check(r.command.capability())
                    .and_then(|_| adapter.execute(&r.command, &context));
                diagnostic.finish(
                    if result.is_ok() {
                        "completed"
                    } else {
                        "failed"
                    },
                    result.as_ref().err().map(|error| error.code),
                );
                result
            })
            .await
            .map_err(|_| Error::execution())?
        });
        accepted();
        tokio::time::timeout(Duration::from_secs(30), task)
            .await
            .map_err(|_| {
                Error::new(
                    ErrorCode::ExecutionFailed,
                    "Action timed out; execution outcome is unknown. No retry was sent",
                )
            })?
            .map_err(|_| Error::execution())?
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

#[cfg(test)]
mod budget_tests {
    use super::*;
    #[test]
    fn remaining_checks_policy_and_clamps_helper_budget() {
        let context = Context {
            capabilities: vec![Capability::Audio],
            deadline: Instant::now() + Duration::from_secs(60),
            depth: 0,
        };
        assert_eq!(
            context
                .remaining(Capability::Audio, Duration::from_secs(5))
                .unwrap(),
            Duration::from_secs(5)
        );
        assert_eq!(
            context
                .remaining(Capability::Network, Duration::from_secs(5))
                .unwrap_err()
                .code,
            ErrorCode::Forbidden
        );
        let short = Context {
            deadline: Instant::now() + Duration::from_secs(1),
            ..context
        };
        assert!(
            short
                .remaining(Capability::Audio, Duration::from_secs(5))
                .unwrap()
                <= Duration::from_secs(1)
        );
    }
    #[test]
    fn expired_and_nested_over_budget_contexts_cannot_start_helpers() {
        let context = Context {
            capabilities: vec![Capability::Audio],
            deadline: Instant::now() - Duration::from_millis(1),
            depth: 0,
        };
        assert_eq!(
            context
                .remaining(Capability::Audio, Duration::from_secs(5))
                .unwrap_err()
                .code,
            ErrorCode::ExecutionFailed
        );
        let deep = Context {
            deadline: Instant::now() + Duration::from_secs(60),
            depth: 9,
            ..context
        };
        assert!(deep
            .remaining(Capability::Audio, Duration::from_secs(5))
            .is_err());
    }
}
