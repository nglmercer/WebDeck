import { invoke } from "webdeck:commands";
const unsafe = (part) => ["__proto__", "constructor", "prototype"].includes(part);
function reference(path, state) {
    let value = {
        vars: state.vars,
        results: state.results,
        last: state.last,
    };
    for (const part of path.split(".")) {
        if (unsafe(part) ||
            !value ||
            !Object.prototype.hasOwnProperty.call(value, part))
            throw new Error("Unknown workflow result reference");
        value = value[part];
    }
    return value;
}
function resolve(value, state) {
    if (value && typeof value === "object") {
        if (!Array.isArray(value) &&
            Object.keys(value).length === 1 &&
            typeof value.$result === "string")
            return reference(value.$result, state);
        if (Array.isArray(value))
            return value.map((item) => resolve(item, state));
        const result = {};
        for (const key of Object.keys(value)) {
            if (unsafe(key))
                throw new Error("Invalid workflow key");
            result[key] = resolve(value[key], state);
        }
        return result;
    }
    return value;
}
function replace(command, path, value) {
    const parts = path.split(".");
    if (parts.some(unsafe) || path === "type")
        throw new Error("Invalid workflow input reference");
    let target = command;
    for (const part of parts.slice(0, -1)) {
        if (!target || !Object.prototype.hasOwnProperty.call(target, part))
            throw new Error("Invalid workflow input path");
        target = target[part];
    }
    const final = parts[parts.length - 1];
    if (!final || !Object.prototype.hasOwnProperty.call(target, final))
        throw new Error("Invalid workflow input path");
    target[final] = value;
}
async function run(node, state) {
    if (state.depth > 32 || Date.now() >= state.deadlineMs)
        throw new Error("Workflow budget exhausted");
    const child = { ...state, depth: state.depth + 1 };
    switch (node.type) {
        case "command": {
            const command = resolve(node.command, state);
            for (const [path, source] of Object.entries(node.references || {}))
                replace(command, path, reference(source, state));
            return await invoke(command, Math.max(1, state.deadlineMs - Date.now()));
        }
        case "sequence": {
            const results = [];
            for (const step of node.steps) {
                const value = await run(step, child);
                child.last = value;
                child.results.push(value);
                results.push(value);
            }
            return results;
        }
        case "parallel":
            return await Promise.all(node.steps.map((step) => run(step, {
                ...child,
                vars: { ...state.vars },
                results: state.results.slice(),
                last: state.last,
            })));
        case "conditional":
            return await run(resolve(node.condition, state) ? node.if_true : node.if_false, child);
        case "delay": {
            const remaining = state.deadlineMs - Date.now();
            if (node.milliseconds >= remaining)
                throw new Error("Workflow delay exceeds deadline");
            await new Promise((done) => setTimeout(() => done(), node.milliseconds));
            return null;
        }
        case "retry": {
            let failure;
            for (let attempt = 0; attempt < node.attempts; attempt++) {
                try {
                    return await run(node.step, child);
                }
                catch (error) {
                    failure = error;
                }
            }
            throw failure;
        }
        case "timeout":
            return await run(node.step, {
                ...child,
                deadlineMs: Math.min(state.deadlineMs, Date.now() + node.milliseconds),
            });
        case "variable": {
            if (unsafe(node.name))
                throw new Error("Invalid workflow variable");
            const value = resolve(node.value, state);
            state.vars[node.name] = value;
            return value;
        }
        case "result":
            return reference(node.path, state);
        case "script":
            return await invoke({ type: "script", language: "javascript", source: node.source }, Math.max(1, state.deadlineMs - Date.now()));
        case "plugin":
            return await invoke({
                type: "plugin",
                plugin_id: node.plugin_id,
                version: node.version,
                action_id: node.action_id,
                args: resolve(node.args, state),
            }, Math.max(1, state.deadlineMs - Date.now()));
    }
}
export async function execute(workflow, context) {
    return await run(workflow, {
        vars: {},
        results: [],
        last: null,
        deadlineMs: context.deadlineMs,
        depth: 0,
    });
}
