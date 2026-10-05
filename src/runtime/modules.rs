use napi_vm::{Interpreter, Value};

pub(super) fn install(vm: &mut Interpreter) {
    vm.global
        .borrow_mut()
        .set("__webdeckLog", Value::host_function("log.metadata", 7));
    vm.global
        .borrow_mut()
        .set("__webdeckEmit", Value::host_function("events.emit", 6));
    vm.define_module(
        "webdeck:events",
        include_str!("../../runtime/dist/core/events.js").into(),
    );
    vm.define_module("webdeck:storage", "export function get(key) { return __webdeckCapability('storage.pluginGet',{key}); } export function set(key,value) { return __webdeckCapability('storage.pluginSet',{key,value}); }".into());
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
    vm.global
        .borrow_mut()
        .set("__webdeckPlugin", Value::host_function("plugins.invoke", 4));
    vm.define_module(
        "webdeck:plugins",
        "export const invoke = __webdeckPlugin;".into(),
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
    vm.global.borrow_mut().set(
        "__webdeckBuiltin",
        Value::host_function("builtins.invoke", 5),
    );
    vm.define_module(
        "webdeck:builtins",
        "export const invoke = __webdeckBuiltin;".into(),
    );
    vm.define_module(
        "webdeck:builtin:obs",
        include_str!("../../runtime/dist/builtins/obs/index.js").into(),
    );
    vm.define_module(
        "webdeck:builtin:spotify",
        include_str!("../../runtime/dist/builtins/spotify/index.js").into(),
    );
    vm.define_module_alias(
        "webdeck:main",
        "./builtins/obs/index.js",
        "webdeck:builtin:obs",
    );
    vm.define_module_alias(
        "webdeck:main",
        "./builtins/spotify/index.js",
        "webdeck:builtin:spotify",
    );
    vm.define_module(
        "webdeck:commands",
        "export const invoke = __webdeckInvoke;".into(),
    );
    vm.define_module(
        "webdeck:workflows",
        include_str!("../../runtime/dist/core/workflows.js").into(),
    );
    vm.define_module_alias("webdeck:main", "./core/workflows.js", "webdeck:workflows");
    vm.define_module(
        "webdeck:main",
        include_str!("../../runtime/dist/main.js").into(),
    );
}
