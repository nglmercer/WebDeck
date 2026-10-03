use super::{bridge::Bridge, errors, modules};
use crate::{
    contracts::Capability,
    domain::{Error, Result},
    executor::Context,
};
use napi_vm::{value_from_json, value_to_json, Interpreter};
use serde_json::{json, Value};
use std::{
    rc::Rc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

pub(super) fn guest_context(context: &Context) -> Result<napi_vm::Value> {
    let remaining = context
        .deadline
        .saturating_duration_since(std::time::Instant::now());
    value_from_json(&json!({
        "capabilities":context.capabilities,
        "depth":context.depth,
        "deadlineMs":SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64 + remaining.as_millis() as u64,
    })).map_err(|_| errors::failed())
}
pub(super) fn boot(bridge: Rc<Bridge>, context: &Context) -> Result<Interpreter> {
    let mut vm = Interpreter::with_builtins();
    vm.set_host_bridge(bridge);
    vm.set_fuel_budget(1_000_000);
    vm.set_loop_budget(100_000);
    vm.set_execution_timeout(Some(
        context
            .deadline
            .saturating_duration_since(std::time::Instant::now())
            .min(Duration::from_secs(30)),
    ));
    modules::install(&mut vm);
    vm.eval_source("import 'webdeck:main';")
        .map_err(|_| errors::failed())?;
    vm.global
        .borrow_mut()
        .set("__webdeckContext", guest_context(context)?);
    vm.eval_source("globalThis.ctx = Object.freeze({ invoke: __webdeckInvoke, log: () => {}, signal: Object.freeze({ get aborted() { return Date.now() >= __webdeckContext.deadlineMs; } }), deadlineMs: __webdeckContext.deadlineMs }); globalThis.invoke = __webdeckInvoke;").map_err(|_| errors::failed())?;
    Ok(vm)
}
pub(super) fn script(bridge: Rc<Bridge>, source: &str, context: &Context) -> Result<Value> {
    context.check(Capability::Script)?;
    if source.len() > 65536 {
        return Err(Error::invalid());
    }
    let mut vm = boot(bridge.clone(), context)?;
    // Fresh global scope per script; nested calls still run inside the accepted root.
    let value = vm.eval_source(source).map_err(|_| {
        bridge
            .error
            .borrow_mut()
            .take()
            .unwrap_or_else(errors::failed)
    })?;
    if matches!(value, napi_vm::Value::Undefined) {
        return Ok(Value::Null);
    }
    value_to_json(&mut vm, &value).map_err(|_| errors::failed())
}
