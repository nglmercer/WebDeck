use super::capabilities::Metrics;
use crate::{contracts::Capability, domain::Error, executor::Context};
use napi_vm::{value_from_json, HostBridge, Value, VmErr};
use std::{cell::RefCell, sync::Arc};

pub(super) struct Bridge {
    pub context: RefCell<Option<Context>>,
    pub error: RefCell<Option<Error>>,
    pub metrics: Arc<dyn Metrics>,
}
impl HostBridge for Bridge {
    fn call_host(&self, id: usize, args: Vec<Value>) -> Result<Value, VmErr> {
        let result = (|| {
            let context = self.context.borrow();
            let context = context.as_ref().ok_or_else(super::errors::unavailable)?;
            context.check(Capability::Read)?;
            if id != 0 || !args.is_empty() {
                return Err(super::errors::failed());
            }
            let value = self.metrics.usage(context)?;
            context.check(Capability::Read)?;
            Ok(value)
        })();
        match result {
            Ok(value) => value_from_json(&value),
            Err(error) => {
                *self.error.borrow_mut() = Some(error);
                Err(VmErr::Msg("Host capability failed".into()))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Result;
    use serde_json::json;
    use std::time::{Duration, Instant};
    struct ForbiddenMetrics;
    impl Metrics for ForbiddenMetrics {
        fn usage(&self, _: &Context) -> Result<serde_json::Value> {
            panic!("unauthorized primitive must not run")
        }
    }
    #[test]
    fn host_boundary_rechecks_context_even_without_executor_or_js_checks() {
        let bridge = Bridge {
            context: RefCell::new(None),
            error: RefCell::new(None),
            metrics: Arc::new(ForbiddenMetrics),
        };
        assert!(bridge.call_host(0, vec![]).is_err());
        *bridge.context.borrow_mut() = Some(Context {
            capabilities: vec![],
            deadline: Instant::now() + Duration::from_secs(5),
            depth: 0,
        });
        assert!(bridge.call_host(0, vec![]).is_err());
        assert_eq!(
            bridge.error.borrow().as_ref().unwrap().code,
            crate::contracts::ErrorCode::Forbidden
        );
        *bridge.context.borrow_mut() = Some(Context {
            capabilities: vec![Capability::Read],
            deadline: Instant::now() - Duration::from_secs(1),
            depth: 0,
        });
        assert!(bridge.call_host(0, vec![]).is_err());
    }
    struct FixedMetrics;
    impl Metrics for FixedMetrics {
        fn usage(&self, _: &Context) -> Result<serde_json::Value> {
            Ok(json!({"cpu_percent":25}))
        }
    }
    #[test]
    fn metrics_parity_uses_the_same_fake_primitive() {
        let context = Context {
            capabilities: vec![Capability::Read],
            deadline: Instant::now() + Duration::from_secs(5),
            depth: 0,
        };
        let metrics = Arc::new(FixedMetrics);
        let native_value = metrics.usage(&context).unwrap();
        let runtime = crate::runtime::VmRuntime::new(metrics).unwrap();
        assert_eq!(
            runtime
                .invoke(&crate::contracts::Command::Usage, &context)
                .unwrap(),
            native_value
        );
    }
}
