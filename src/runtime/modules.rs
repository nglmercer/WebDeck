use napi_vm::{Interpreter, Value};

pub(super) fn install(vm: &mut Interpreter) {
    vm.global.borrow_mut().set(
        "__webdeckMetricsUsage",
        Value::host_function("metrics.usage", 0),
    );
    vm.global
        .borrow_mut()
        .set("__webdeckCapability", Value::host_function("capability", 1));
    vm.global.borrow_mut().set(
        "__webdeckInvoke",
        Value::host_function("commands.invoke", 2),
    );
    vm.global.borrow_mut().set(
        "__webdeckScript",
        Value::host_function("scripts.execute", 3),
    );
    vm.define_module(
        "webdeck:scripts",
        "export const execute = __webdeckScript;".into(),
    );
    vm.define_module(
        "webdeck:host",
        "export const call = __webdeckCapability;".into(),
    );
    vm.define_module(
        "webdeck:network",
        "export function fetch(input) { return __webdeckCapability('network.fetch', input); }"
            .into(),
    );
    vm.define_module(
        "webdeck:metrics",
        "export const usage = __webdeckMetricsUsage;".into(),
    );
    vm.define_module(
        "webdeck:main",
        include_str!("../../runtime/dist/main.js").into(),
    );
}
