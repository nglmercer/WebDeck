use super::capabilities::{required_capability, CapabilityHost, Metrics};
use crate::{contracts::Capability, domain::Error, executor::Context};
use napi_vm::{
    value_from_json, value_to_json, HostBridge, HostCallback, Interpreter, Value, VmErr,
};
use std::{cell::RefCell, sync::Arc};

pub(super) struct Bridge {
    pub this: std::cell::OnceCell<std::rc::Weak<Bridge>>,
    pub context: RefCell<Option<Context>>,
    pub error: RefCell<Option<Error>>,
    pub metrics: Arc<dyn Metrics>,
    pub host: Arc<dyn CapabilityHost>,
}
impl HostBridge for Bridge {
    fn call_host_with_interp(
        &self,
        id: usize,
        _this: Value,
        args: Vec<Value>,
        handler: &mut dyn FnMut(
            &mut Interpreter,
            HostCallback,
        ) -> std::result::Result<Value, VmErr>,
        interp: &mut Interpreter,
    ) -> std::result::Result<Value, VmErr> {
        if id == 0 {
            return self.call_host(id, args);
        }
        if id == 2 || id == 3 {
            let result = self.dynamic_call(id, args, handler, interp);
            return self.guest_result(result);
        }
        let result = (|| {
            if id != 1 || args.len() != 2 {
                return Err(super::errors::failed());
            }
            let operation = value_to_json(interp, &args[0]).map_err(|_| super::errors::failed())?;
            let operation = operation.as_str().ok_or_else(super::errors::failed)?;
            let input = value_to_json(interp, &args[1]).map_err(|_| super::errors::failed())?;
            let context = self.context.borrow();
            let context = context.as_ref().ok_or_else(super::errors::unavailable)?;
            let capability = required_capability(operation)?;
            context.check(capability)?;
            let value = self.host.call(operation, &input, context)?;
            context.check(capability)?;
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

impl Bridge {
    fn guest_result(
        &self,
        result: crate::domain::Result<serde_json::Value>,
    ) -> std::result::Result<Value, VmErr> {
        match result {
            Ok(value) => value_from_json(&value),
            Err(error) => {
                *self.error.borrow_mut() = Some(error);
                Err(VmErr::Msg("Runtime operation denied or failed".into()))
            }
        }
    }
    fn dynamic_call(
        &self,
        id: usize,
        args: Vec<Value>,
        handler: &mut dyn FnMut(
            &mut Interpreter,
            HostCallback,
        ) -> std::result::Result<Value, VmErr>,
        interp: &mut Interpreter,
    ) -> crate::domain::Result<serde_json::Value> {
        if args.len() != 1 {
            return Err(crate::domain::Error::invalid());
        }
        let original = self
            .context
            .borrow()
            .clone()
            .ok_or_else(super::errors::unavailable)?;
        if id == 3 {
            original.check(Capability::Script)?;
            let source = value_to_json(interp, &args[0]).map_err(|_| super::errors::failed())?;
            let source = source.as_str().ok_or_else(crate::domain::Error::invalid)?;
            let bridge = self
                .this
                .get()
                .and_then(|weak| weak.upgrade())
                .ok_or_else(super::errors::unavailable)?;
            return super::sandbox::script(bridge, source, &original);
        }
        let input = value_to_json(interp, &args[0]).map_err(|_| super::errors::failed())?;
        let command: crate::contracts::Command =
            serde_json::from_value(input).map_err(|_| crate::domain::Error::invalid())?;
        crate::domain::validate_command(&command)?;
        let nested = original.nested();
        nested.check(command.capability())?;
        let callback = interp
            .global
            .borrow()
            .get("__webdeckDispatch")
            .ok_or_else(super::errors::unavailable)?;
        let context = super::sandbox::guest_context(&nested)?;
        *self.context.borrow_mut() = Some(nested);
        let result = handler(
            interp,
            HostCallback {
                callback,
                this_value: Value::Undefined,
                args: vec![args[0].clone(), context],
                kind: napi_vm::HostCallbackKind::Call,
            },
        );
        *self.context.borrow_mut() = Some(original);
        let result = result.map_err(|_| {
            self.error
                .borrow_mut()
                .take()
                .unwrap_or_else(super::errors::failed)
        })?;
        if matches!(result, Value::Undefined) {
            return Ok(serde_json::Value::Null);
        }
        value_to_json(interp, &result).map_err(|_| super::errors::failed())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Result;
    use crate::runtime::capabilities::UnavailableHost;
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
            this: std::cell::OnceCell::new(),
            context: RefCell::new(None),
            error: RefCell::new(None),
            metrics: Arc::new(ForbiddenMetrics),
            host: Arc::new(UnavailableHost),
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
