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
}
impl VmRuntime {
    /// Boot synchronously so startup failure cannot silently select native dispatch.
    pub fn new(metrics: Arc<dyn Metrics>) -> Result<Self> {
        Self::with_host(metrics, Arc::new(UnavailableHost))
    }
    pub fn with_host(metrics: Arc<dyn Metrics>, host: Arc<dyn CapabilityHost>) -> Result<Self> {
        let (sender, receiver) = mpsc::sync_channel(QUEUE_CAPACITY);
        let (ready, booted) = mpsc::channel();
        let thread = thread::Builder::new()
            .name("webdeck-vm".into())
            .spawn(move || {
                let initialized = Machine::boot(metrics, host.clone());
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
                                Request::Shutdown => break,
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
}
impl Machine {
    fn boot(metrics: Arc<dyn Metrics>, host: Arc<dyn CapabilityHost>) -> Result<Self> {
        let bridge = Rc::new(Bridge {
            this: std::cell::OnceCell::new(),
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
        let invoke = Interpreter::compile(
            "__webdeckRuntime.invoke(__webdeckCommand, Object.freeze({ capabilities: Object.freeze(__webdeckContext.capabilities), deadlineMs: __webdeckContext.deadlineMs, depth: __webdeckContext.depth }));"
        ).map_err(|_| errors::unavailable())?;
        Ok(Self { vm, bridge, invoke })
    }
    fn invoke(&mut self, command: &Command, context: &Context) -> Result<Value> {
        context.check(command.capability())?;
        let remaining = context.remaining(command.capability(), Duration::from_secs(30))?;
        self.vm.set_execution_timeout(Some(remaining));
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
