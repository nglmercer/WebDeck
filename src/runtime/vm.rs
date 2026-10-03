use super::{
    bridge::Bridge,
    capabilities::{CapabilityHost, Metrics, UnavailableHost},
    errors, modules,
};
use crate::{
    contracts::{Command, ErrorCode},
    domain::{self, Error, Result},
    executor::Context,
};
use napi_vm::{value_from_json, value_to_json, Interpreter, PreparedProgram, Value as GuestValue};
use serde_json::{json, Value};
use std::{
    cell::RefCell,
    rc::Rc,
    sync::{mpsc, Arc, Mutex},
    thread::{self, JoinHandle},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

const QUEUE_CAPACITY: usize = 16;
enum Request {
    Invoke {
        command: Command,
        context: Context,
        reply: mpsc::Sender<Result<Value>>,
    },
    Management {
        plugins: Option<Vec<super::plugins::RuntimePlugin>>,
        reply: mpsc::Sender<Result<Value>>,
    },
    PluginEnabled {
        id: String,
        enabled: bool,
        reply: mpsc::Sender<Result<Value>>,
    },
    Shutdown,
}
struct Owner {
    sender: Option<mpsc::SyncSender<Request>>,
    thread: Option<JoinHandle<()>>,
}
/// Only the owner thread constructs, uses and drops guest values and the interpreter.
/// The handle transports plain Rust data through a bounded queue.
pub struct VmRuntime {
    owner: Mutex<Owner>,
    events: tokio::sync::broadcast::Sender<Value>,
}
impl VmRuntime {
    /// Boot synchronously so startup failure cannot silently select native dispatch.
    pub fn new(metrics: Arc<dyn Metrics>) -> Result<Self> {
        Self::with_host(metrics, Arc::new(UnavailableHost))
    }
    pub fn with_host(metrics: Arc<dyn Metrics>, host: Arc<dyn CapabilityHost>) -> Result<Self> {
        Self::with_plugins(metrics, host, vec![])
    }
    pub fn with_plugins(
        metrics: Arc<dyn Metrics>,
        host: Arc<dyn CapabilityHost>,
        plugins: Vec<super::plugins::RuntimePlugin>,
    ) -> Result<Self> {
        let plugins = plugins
            .into_iter()
            .chain(super::plugins::builtins())
            .collect();
        let (events, _) = tokio::sync::broadcast::channel(64);
        let event_sender = events.clone();
        let (sender, receiver) = mpsc::sync_channel(QUEUE_CAPACITY);
        let (ready, booted) = mpsc::channel();
        let thread = thread::Builder::new()
            .name("webdeck-vm".into())
            .spawn(move || {
                let initialized = Machine::boot(metrics, host.clone(), plugins, event_sender);
                match initialized {
                    Ok(mut machine) => {
                        if ready.send(Ok(())).is_err() {
                            return;
                        }
                        for request in receiver {
                            match request {
                                Request::Invoke {
                                    command,
                                    context,
                                    reply,
                                } => {
                                    let _ = reply.send(machine.invoke(&command, &context));
                                }
                                Request::Management { plugins, reply } => {
                                    let _ = reply.send(machine.management(plugins));
                                }
                                Request::PluginEnabled { id, enabled, reply } => {
                                    let result = if !id.starts_with("builtin.")
                                        && machine.bridge.plugins.borrow().contains_key(&id)
                                    {
                                        machine.unload();
                                        if enabled {
                                            machine
                                                .bridge
                                                .disabled_plugins
                                                .borrow_mut()
                                                .remove(&id);
                                        } else {
                                            machine.bridge.disabled_plugins.borrow_mut().insert(id);
                                        }
                                        let _ = machine.bridge.events.send(
                                            json!({"api_version":2,"type":"runtime.reloaded"}),
                                        );
                                        machine.management(None)
                                    } else {
                                        Err(Error::invalid())
                                    };
                                    let _ = reply.send(result);
                                }
                                Request::Shutdown => {
                                    machine.unload();
                                    break;
                                }
                            }
                        }
                        host.shutdown();
                    }
                    Err(error) => {
                        let _ = ready.send(Err(error));
                    }
                }
            })
            .map_err(|_| errors::unavailable())?;
        match booted.recv().unwrap_or_else(|_| Err(errors::unavailable())) {
            Ok(()) => Ok(Self {
                events,
                owner: Mutex::new(Owner {
                    sender: Some(sender),
                    thread: Some(thread),
                }),
            }),
            Err(error) => {
                let _ = thread.join();
                Err(error)
            }
        }
    }
    pub fn invoke(&self, command: &Command, context: &Context) -> Result<Value> {
        context.check(command.capability())?;
        domain::validate_command(command)?;
        let (reply, response) = mpsc::channel();
        {
            let owner = self.owner.lock().map_err(|_| errors::unavailable())?;
            let sender = owner.sender.as_ref().ok_or_else(errors::unavailable)?;
            sender
                .try_send(Request::Invoke {
                    command: command.clone(),
                    context: context.clone(),
                    reply,
                })
                .map_err(|error| match error {
                    mpsc::TrySendError::Full(_) => {
                        Error::new(ErrorCode::CapacityExhausted, "Runtime queue full")
                    }
                    mpsc::TrySendError::Disconnected(_) => errors::unavailable(),
                })?;
        }
        // Keep the Executor's accepted root owned until the VM/native call finishes.
        // Executor times out observation; a host side effect is never claimed undone.
        response.recv().map_err(|_| errors::unavailable())?
    }
    pub fn plugin_enabled(&self, id: &str, enabled: bool) -> Result<Value> {
        let (reply, response) = mpsc::channel();
        {
            let owner = self.owner.lock().map_err(|_| errors::unavailable())?;
            owner
                .sender
                .as_ref()
                .ok_or_else(errors::unavailable)?
                .try_send(Request::PluginEnabled {
                    id: id.into(),
                    enabled,
                    reply,
                })
                .map_err(|_| {
                    Error::new(ErrorCode::CapacityExhausted, "Runtime queue unavailable")
                })?;
        }
        response.recv().map_err(|_| errors::unavailable())?
    }
    pub fn events(&self) -> tokio::sync::broadcast::Receiver<Value> {
        self.events.subscribe()
    }
    pub fn management(&self, plugins: Option<Vec<super::plugins::RuntimePlugin>>) -> Result<Value> {
        let (reply, response) = mpsc::channel();
        {
            let owner = self.owner.lock().map_err(|_| errors::unavailable())?;
            owner
                .sender
                .as_ref()
                .ok_or_else(errors::unavailable)?
                .try_send(Request::Management { plugins, reply })
                .map_err(|_| {
                    Error::new(ErrorCode::CapacityExhausted, "Runtime queue unavailable")
                })?;
        }
        response.recv().map_err(|_| errors::unavailable())?
    }
    /// Executor drains accepted roots before calling this. Repeat calls are harmless.
    pub fn shutdown(&self) {
        let mut owner = self.owner.lock().unwrap_or_else(|error| error.into_inner());
        if let Some(sender) = owner.sender.take() {
            let _ = sender.send(Request::Shutdown);
        }
        if let Some(thread) = owner.thread.take() {
            let _ = thread.join();
        }
    }
}
impl Drop for VmRuntime {
    fn drop(&mut self) {
        self.shutdown();
    }
}
struct Machine {
    vm: Interpreter,
    bridge: Rc<Bridge>,
    invoke: PreparedProgram,
    healthy: bool,
}
impl Machine {
    fn boot(
        metrics: Arc<dyn Metrics>,
        host: Arc<dyn CapabilityHost>,
        plugins: Vec<super::plugins::RuntimePlugin>,
        events: tokio::sync::broadcast::Sender<Value>,
    ) -> Result<Self> {
        let bridge = Rc::new(Bridge {
            events,
            this: std::cell::OnceCell::new(),
            plugins: RefCell::new(
                plugins
                    .into_iter()
                    .map(|plugin| (plugin.manifest.id.clone(), plugin))
                    .collect(),
            ),
            sessions: RefCell::new(std::collections::HashMap::new()),
            builtin: RefCell::new(None),
            disabled_plugins: RefCell::new(std::collections::HashSet::new()),
            active_plugins: RefCell::new(std::collections::HashSet::new()),
            calls: std::cell::Cell::new(0),
            context: RefCell::new(None),
            error: RefCell::new(None),
            metrics,
            host,
        });
        bridge
            .this
            .set(Rc::downgrade(&bridge))
            .map_err(|_| errors::unavailable())?;
        let mut vm = Interpreter::with_builtins();
        vm.set_host_bridge(bridge.clone());
        vm.set_fuel_budget(1_000_000);
        vm.set_loop_budget(100_000);
        vm.set_execution_timeout(Some(Duration::from_secs(5)));
        modules::install(&mut vm);
        vm.eval_source("import 'webdeck:main';")
            .map_err(|_| errors::unavailable())?;
        *bridge.builtin.borrow_mut() = vm.global.borrow().get("__webdeckBuiltinDispatch");
        let invoke = Interpreter::compile(
            "await __webdeckRuntime.invoke(__webdeckCommand, Object.freeze({ capabilities: Object.freeze(__webdeckContext.capabilities), deadlineMs: __webdeckContext.deadlineMs, depth: __webdeckContext.depth }));"
        ).map_err(|_| errors::unavailable())?;
        Ok(Self {
            vm,
            bridge,
            invoke,
            healthy: true,
        })
    }
    fn unload(&mut self) {
        let sessions = std::mem::take(&mut *self.bridge.sessions.borrow_mut());
        for (id, mut vm) in sessions {
            let capabilities = self
                .bridge
                .plugins
                .borrow()
                .get(&id)
                .map(|p| {
                    p.manifest
                        .actions
                        .iter()
                        .flat_map(|a| a.capabilities.iter().copied())
                        .collect::<std::collections::HashSet<_>>()
                        .into_iter()
                        .collect()
                })
                .unwrap_or_default();
            let context = Context {
                principal: Some(id),
                owner_id: "plugin-unload".into(),
                capabilities,
                deadline: std::time::Instant::now() + Duration::from_secs(1),
                depth: 0,
            };
            *self.bridge.context.borrow_mut() = Some(context.clone());
            vm.set_execution_timeout(Some(Duration::from_secs(1)));
            if let Ok(guest) = super::sandbox::guest_context(&context) {
                vm.global.borrow_mut().set("__webdeckContext", guest);
            }
            let _ = super::sandbox::install_context(&mut vm);
            if let Ok(value) = vm.eval_source("typeof __webdeckPlugin.onUnload === 'function' ? __webdeckPlugin.onUnload(ctx) : null") { let _ = super::sandbox::settle(&mut vm, value); }
            self.bridge.host.finish_root(&context);
        }
        *self.bridge.context.borrow_mut() = None;
        *self.bridge.error.borrow_mut() = None;
        self.bridge.calls.set(0);
        self.bridge.host.reload_plugins();
    }
    fn management(&mut self, plugins: Option<Vec<super::plugins::RuntimePlugin>>) -> Result<Value> {
        if let Some(plugins) = plugins {
            self.unload();
            let plugins = plugins
                .into_iter()
                .chain(super::plugins::builtins())
                .collect();
            *self = Self::boot(
                self.bridge.metrics.clone(),
                self.bridge.host.clone(),
                plugins,
                self.bridge.events.clone(),
            )?;
            let _ = self
                .bridge
                .events
                .send(json!({"api_version":2,"type":"runtime.reloaded"}));
        }
        let value = self
            .vm
            .eval_source("__webdeckRuntime.list()")
            .map_err(|_| errors::failed())?;
        let commands = value_to_json(&mut self.vm, &value).map_err(|_| errors::failed())?;
        let mut plugins = self
            .bridge
            .plugins
            .borrow()
            .values()
            .map(|p| p.manifest.clone())
            .collect::<Vec<_>>();
        plugins.sort_by(|a, b| a.id.cmp(&b.id));
        let mut loaded = self
            .bridge
            .sessions
            .borrow()
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        loaded.sort();
        let mut disabled = self
            .bridge
            .disabled_plugins
            .borrow()
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        disabled.sort();
        Ok(
            json!({"disabled_plugins":disabled,"api_version":2,"runtime":"napi-vm","healthy":self.healthy,"commands":commands,"plugins":plugins,"loaded_plugins":loaded,"queue_capacity":QUEUE_CAPACITY}),
        )
    }
    fn invoke(&mut self, command: &Command, context: &Context) -> Result<Value> {
        if !self.healthy {
            return Err(errors::unavailable());
        }
        context.check(command.capability())?;
        let remaining = context.remaining(command.capability(), Duration::from_secs(30))?;
        self.vm.set_execution_timeout(Some(remaining));
        self.bridge.calls.set(0);
        *self.bridge.context.borrow_mut() = Some(context.clone());
        *self.bridge.error.borrow_mut() = None;
        let result = (|| {
            let input = serde_json::to_value(command).map_err(|_| Error::invalid())?;
            let guest = value_from_json(&input).map_err(|_| errors::failed())?;
            let guest_context = value_from_json(&json!({
                "capabilities": context.capabilities,
                "deadlineMs": SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64 + remaining.as_millis() as u64,
                "depth": context.depth,
            }))
            .map_err(|_| errors::failed())?;
            self.vm.global.borrow_mut().set("__webdeckCommand", guest);
            self.vm
                .global
                .borrow_mut()
                .set("__webdeckContext", guest_context);
            let result = self
                .vm
                .execute(&self.invoke)
                .map_err(|_| errors::failed())?;
            let result = value_to_json(&mut self.vm, &result).map_err(|_| errors::failed())?;
            context.check(command.capability())?;
            Ok(result)
        })();
        if result.is_err() && !self.vm.jobs.borrow().is_empty() {
            self.healthy = false;
        }
        self.bridge.host.finish_root(context);
        self.vm
            .global
            .borrow_mut()
            .set("__webdeckCommand", GuestValue::Undefined);
        self.vm
            .global
            .borrow_mut()
            .set("__webdeckContext", GuestValue::Undefined);
        *self.bridge.context.borrow_mut() = None;
        let error = self.bridge.error.borrow_mut().take();
        match (result, error) {
            (Err(_), Some(error)) => Err(error),
            (result, _) => result,
        }
    }
}
