use super::capabilities::{required_capability, CapabilityHost, Metrics};
use super::errors;
use crate::{contracts::Capability, domain::Error, executor::Context};
use napi_vm::{
    value_from_json, value_to_json, HostBridge, HostCallback, Interpreter, Value, VmErr,
};
use std::{cell::RefCell, sync::Arc};

pub(super) struct Bridge {
    pub events: tokio::sync::broadcast::Sender<serde_json::Value>,
    pub plugins: RefCell<std::collections::HashMap<String, super::plugins::RuntimePlugin>>,
    pub sessions: RefCell<std::collections::HashMap<String, Interpreter>>,
    pub builtin: RefCell<Option<Value>>,
    pub disabled_plugins: RefCell<std::collections::HashSet<String>>,
    pub active_plugins: RefCell<std::collections::HashSet<String>>,
    pub calls: std::cell::Cell<usize>,
    pub this: std::cell::OnceCell<std::rc::Weak<Bridge>>,
    pub context: RefCell<Option<Context>>,
    pub error: RefCell<Option<Error>>,
    pub metrics: Arc<dyn Metrics>,
    pub host: Arc<dyn CapabilityHost>,
}
impl HostBridge for Bridge {
    fn trace_roots(&self, values: &mut Vec<Value>, _envs: &mut Vec<napi_vm::interpreter::Env>) {
        if let Some(value) = self.builtin.borrow().as_ref() {
            values.push(value.clone());
        }
    }
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
        self.calls.set(self.calls.get() + 1);
        if self.calls.get() > 1024 {
            return self.guest_result(Err(super::errors::failed()));
        }
        if id == 7 {
            let result = (|| {
                if args.len() != 1 {
                    return Err(Error::invalid());
                }
                let value = value_to_json(interp, &args[0]).map_err(|_| errors::failed())?;
                let message = value
                    .as_str()
                    .filter(|s| s.len() <= 4096)
                    .ok_or_else(Error::invalid)?;
                let context = self.context.borrow();
                let context = context.as_ref().ok_or_else(errors::unavailable)?;
                context.check_budget()?;
                if std::env::var("WEBDECK_DIAGNOSTICS").as_deref() == Ok("1") {
                    eprintln!(
                        "{}",
                        serde_json::json!({"event":"runtime_log","message_bytes":message.len(),"depth":context.depth})
                    );
                }
                Ok(serde_json::Value::Null)
            })();
            return self.guest_result(result);
        }
        if id == 6 {
            let result = (|| {
                if args.len() != 1 {
                    return Err(Error::invalid());
                }
                let event = value_to_json(interp, &args[0]).map_err(|_| errors::failed())?;
                let context = self.context.borrow();
                context
                    .as_ref()
                    .ok_or_else(errors::unavailable)?
                    .check(Capability::Read)?;
                crate::domain::validate("RuntimeEvent", &event)?;
                let _ = self.events.send(event);
                Ok(serde_json::Value::Null)
            })();
            return self.guest_result(result);
        }
        if id == 5 {
            return self.guest_result(self.builtin_call(args, handler, interp));
        }
        if id == 4 {
            return self.guest_result(self.plugin_call(args, interp));
        }
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
    fn builtin_call(
        &self,
        args: Vec<Value>,
        handler: &mut dyn FnMut(
            &mut Interpreter,
            HostCallback,
        ) -> std::result::Result<Value, VmErr>,
        interp: &mut Interpreter,
    ) -> crate::domain::Result<serde_json::Value> {
        use crate::{
            contracts::Command,
            domain::{validate_command, Error},
        };
        if args.len() != 1 {
            return Err(Error::invalid());
        }
        let input = value_to_json(interp, &args[0]).map_err(|_| errors::failed())?;
        let command: Command = serde_json::from_value(input).map_err(|_| Error::invalid())?;
        validate_command(&command)?;
        let principal = match command {
            Command::Obs { .. } => "builtin.obs",
            Command::Spotify { .. } => "builtin.spotify",
            _ => return Err(Error::invalid()),
        };
        let original = self
            .context
            .borrow()
            .clone()
            .ok_or_else(errors::unavailable)?;
        original.check(Capability::Network)?;
        let mut scoped = original.clone();
        scoped.principal = Some(principal.into());
        let callback = self
            .builtin
            .borrow()
            .clone()
            .ok_or_else(errors::unavailable)?;
        *self.context.borrow_mut() = Some(scoped);
        let result = handler(
            interp,
            HostCallback {
                callback,
                this_value: Value::Undefined,
                args,
                kind: napi_vm::HostCallbackKind::Call,
            },
        );
        let result = result
            .map_err(|_| {
                self.error
                    .borrow_mut()
                    .take()
                    .unwrap_or_else(errors::failed)
            })
            .and_then(|value| super::sandbox::settle(interp, value))
            .and_then(|value| {
                original.check_budget()?;
                Ok(value)
            });
        *self.context.borrow_mut() = Some(original);
        let result = result?;
        value_to_json(interp, &result).map_err(|_| errors::failed())
    }
    fn plugin_call(
        &self,
        args: Vec<Value>,
        interp: &mut Interpreter,
    ) -> crate::domain::Result<serde_json::Value> {
        use crate::{
            contracts::Command,
            domain::{validate_command, Error},
        };
        if args.len() != 1 {
            return Err(Error::invalid());
        }
        let value = value_to_json(interp, &args[0]).map_err(|_| errors::failed())?;
        let command: Command = serde_json::from_value(value).map_err(|_| Error::invalid())?;
        validate_command(&command)?;
        let Command::Plugin {
            plugin_id,
            version,
            action_id,
            args,
        } = command
        else {
            return Err(Error::invalid());
        };
        let original = self
            .context
            .borrow()
            .clone()
            .ok_or_else(errors::unavailable)?;
        original.check(Capability::Plugin)?;
        if self.disabled_plugins.borrow().contains(&plugin_id) {
            return Err(Error::new(
                crate::contracts::ErrorCode::ExecutionFailed,
                "Plugin disabled after an execution failure; reload plugins to retry",
            ));
        }
        let plugin = self
            .plugins
            .borrow()
            .get(&plugin_id)
            .filter(|p| p.manifest.version == version)
            .cloned()
            .ok_or_else(Error::invalid)?;
        let action = plugin
            .manifest
            .actions
            .iter()
            .find(|a| a.id == action_id)
            .ok_or_else(Error::invalid)?;
        for capability in &action.capabilities {
            original.check(*capability)?;
        }
        for (name, value) in &args {
            let field = action.arguments.get(name).ok_or_else(Error::invalid)?;
            super::plugins::validate_type(field, value)?;
        }
        if action
            .arguments
            .iter()
            .any(|(name, field)| field.required && !args.contains_key(name))
        {
            return Err(Error::invalid());
        }
        let mut narrowed = original.nested();
        narrowed.check_budget()?;
        narrowed.principal = Some(plugin_id.clone());
        narrowed
            .capabilities
            .retain(|c| action.capabilities.contains(c));
        *self.context.borrow_mut() = Some(narrowed.clone());
        let result = (|| {
            let args = serde_json::to_value(&args).map_err(|_| Error::invalid())?;
            if plugin.manifest.backend == "trusted_process" {
                return self
                    .host
                    .trusted_plugin(&plugin, &action_id, &args, &narrowed);
            }
            let bridge = self
                .this
                .get()
                .and_then(|weak| weak.upgrade())
                .ok_or_else(errors::unavailable)?;
            if !self.active_plugins.borrow_mut().insert(plugin_id.clone()) {
                return Err(errors::failed());
            }
            let cached = self.sessions.borrow_mut().remove(&plugin_id);
            let initialized = (|| {
                if let Some(vm) = cached {
                    return Ok(vm);
                }
                let mut vm = super::sandbox::boot(bridge, &narrowed)?;
                vm.define_module(
                    "webdeck:plugin",
                    plugin.source.clone().ok_or_else(Error::invalid)?,
                );
                vm.eval_source("import * as plugin from 'webdeck:plugin'; globalThis.__webdeckPlugin = plugin;").map_err(|_| errors::failed())?;
                let loaded = vm.eval_source("typeof __webdeckPlugin.onLoad === 'function' ? __webdeckPlugin.onLoad(ctx) : null").map_err(|_| errors::failed())?;
                super::sandbox::settle(&mut vm, loaded)?;
                Ok(vm)
            })();
            let mut vm = match initialized {
                Ok(vm) => vm,
                Err(error) => {
                    self.active_plugins.borrow_mut().remove(&plugin_id);
                    return Err(error);
                }
            };
            let result = (|| {
                vm.set_fuel_budget(1_000_000);
                vm.set_loop_budget(100_000);
                vm.set_execution_timeout(Some(
                    narrowed
                        .deadline
                        .saturating_duration_since(std::time::Instant::now()),
                ));
                vm.global.borrow_mut().set(
                    "__webdeckContext",
                    super::sandbox::guest_context(&narrowed)?,
                );
                super::sandbox::install_context(&mut vm)?;
                vm.global.borrow_mut().set(
                    "__webdeckAction",
                    value_from_json(&serde_json::json!(action_id)).map_err(|_| errors::failed())?,
                );
                vm.global.borrow_mut().set(
                    "__webdeckArguments",
                    value_from_json(&args).map_err(|_| errors::failed())?,
                );
                let value = vm
                    .eval_source(
                        "__webdeckPlugin.invoke_action(__webdeckAction, __webdeckArguments, ctx)",
                    )
                    .map_err(|_| {
                        self.error
                            .borrow_mut()
                            .take()
                            .unwrap_or_else(errors::failed)
                    })?;
                let value = super::sandbox::settle(&mut vm, value)?;
                value_to_json(&mut vm, &value).map_err(|_| errors::failed())
            })();
            self.active_plugins.borrow_mut().remove(&plugin_id);
            if result.is_ok() {
                self.sessions.borrow_mut().insert(plugin_id.clone(), vm);
            }
            result
        })();
        *self.context.borrow_mut() = Some(original);
        if result
            .as_ref()
            .is_err_and(|e| e.code == crate::contracts::ErrorCode::ExecutionFailed)
        {
            self.disabled_plugins.borrow_mut().insert(plugin_id.clone());
        }
        let value = result?;
        narrowed.check_budget()?;
        if super::plugins::validate_type(&action.result, &value).is_err() {
            self.disabled_plugins.borrow_mut().insert(plugin_id.clone());
            return Err(Error::new(
                crate::contracts::ErrorCode::ExecutionFailed,
                "Plugin result contract mismatch",
            ));
        }
        Ok(
            serde_json::json!({"plugin_id":plugin_id,"version":version,"action_id":action_id,"value":value}),
        )
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
        if args.len() != 1 && !(id == 2 && args.len() == 2) {
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
        let mut nested = original.nested();
        if args.len() == 2 {
            let budget = value_to_json(interp, &args[1])
                .map_err(|_| errors::failed())?
                .as_u64()
                .filter(|ms| *ms > 0)
                .ok_or_else(crate::domain::Error::invalid)?;
            nested.deadline = nested.deadline.min(
                std::time::Instant::now() + std::time::Duration::from_millis(budget.min(30000)),
            );
        }
        nested.check(command.capability())?;
        let callback = interp
            .global
            .borrow()
            .get("__webdeckDispatch")
            .ok_or_else(super::errors::unavailable)?;
        let context = super::sandbox::guest_context(&nested)?;
        *self.context.borrow_mut() = Some(nested.clone());
        let result = handler(
            interp,
            HostCallback {
                callback,
                this_value: Value::Undefined,
                args: vec![args[0].clone(), context],
                kind: napi_vm::HostCallbackKind::Call,
            },
        );
        let result = result
            .map_err(|_| {
                self.error
                    .borrow_mut()
                    .take()
                    .unwrap_or_else(errors::failed)
            })
            .and_then(|value| super::sandbox::settle(interp, value))
            .and_then(|value| {
                nested.check_budget()?;
                Ok(value)
            });
        *self.context.borrow_mut() = Some(original);
        let result = result?;
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
            events: tokio::sync::broadcast::channel(64).0,
            disabled_plugins: RefCell::new(std::collections::HashSet::new()),
            active_plugins: RefCell::new(std::collections::HashSet::new()),
            this: std::cell::OnceCell::new(),
            plugins: RefCell::new(std::collections::HashMap::new()),
            sessions: RefCell::new(std::collections::HashMap::new()),
            builtin: RefCell::new(None),
            calls: std::cell::Cell::new(0),
            context: RefCell::new(None),
            error: RefCell::new(None),
            metrics: Arc::new(ForbiddenMetrics),
            host: Arc::new(UnavailableHost),
        };
        assert!(bridge.call_host(0, vec![]).is_err());
        *bridge.context.borrow_mut() = Some(Context {
            principal: None,
            owner_id: "test-root".into(),
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
            principal: None,
            owner_id: "test-root".into(),
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
            principal: None,
            owner_id: "test-root".into(),
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
