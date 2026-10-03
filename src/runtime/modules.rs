use napi_vm::{Interpreter, Value};

pub(super) fn install(vm: &mut Interpreter) {
    vm.global.borrow_mut().set(
        "__webdeckMetricsUsage",
        Value::host_function("metrics.usage", 0),
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
