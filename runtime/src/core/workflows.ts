import { invoke } from "webdeck:commands";
import type { WorkflowNode } from "../generated/contracts";
type State = {
  vars: Record<string, unknown>;
  results: unknown[];
  last: unknown;
  deadlineMs: number;
  depth: number;
};
const unsafe = (part: string) =>
  ["__proto__", "constructor", "prototype"].includes(part);
function reference(path: string, state: State): unknown {
  let value: any = {
    vars: state.vars,
    results: state.results,
    last: state.last,
  };
  for (const part of path.split(".")) {
    if (
      unsafe(part) ||
      !value ||
      !Object.prototype.hasOwnProperty.call(value, part)
    )
      throw new Error("Unknown workflow result reference");
    value = value[part];
  }
  return value;
}
function resolve(value: any, state: State): any {
  if (value && typeof value === "object") {
    if (
      !Array.isArray(value) &&
      Object.keys(value).length === 1 &&
      typeof value.$result === "string"
    )
      return reference(value.$result, state);
    if (Array.isArray(value)) return value.map((item) => resolve(item, state));
    const result: Record<string, unknown> = {};
    for (const key of Object.keys(value)) {
      if (unsafe(key)) throw new Error("Invalid workflow key");
      result[key] = resolve(value[key], state);
    }
    return result;
  }
  return value;
}
function replace(command: any, path: string, value: unknown): void {
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
async function run(node: WorkflowNode, state: State): Promise<unknown> {
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
      return await Promise.all(
        node.steps.map((step) =>
          run(step, {
            ...child,
            vars: { ...state.vars },
            results: state.results.slice(),
            last: state.last,
          }),
        ),
      );
    case "conditional":
      return await run(
        resolve(node.condition, state) ? node.if_true : node.if_false,
        child,
      );
    case "delay": {
      const remaining = state.deadlineMs - Date.now();
      if (node.milliseconds >= remaining)
        throw new Error("Workflow delay exceeds deadline");
      await new Promise<void>((done) =>
        setTimeout(() => done(), node.milliseconds),
      );
      return null;
    }
    case "retry": {
      let failure: unknown;
      for (let attempt = 0; attempt < node.attempts; attempt++) {
        try {
          return await run(node.step, child);
        } catch (error) {
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
      if (unsafe(node.name)) throw new Error("Invalid workflow variable");
      const value = resolve(node.value, state);
      state.vars[node.name] = value;
      return value;
    }
    case "result":
      return reference(node.path, state);
    case "script":
      return await invoke(
        { type: "script", language: "javascript", source: node.source },
        Math.max(1, state.deadlineMs - Date.now()),
      );
    case "plugin":
      return await invoke(
        {
          type: "plugin",
          plugin_id: node.plugin_id,
          version: node.version,
          action_id: node.action_id,
          args: resolve(node.args, state),
        },
        Math.max(1, state.deadlineMs - Date.now()),
      );
  }
}
export async function execute(
  workflow: WorkflowNode,
  context: { deadlineMs: number },
): Promise<unknown> {
  return await run(workflow, {
    vars: {},
    results: [],
    last: null,
    deadlineMs: context.deadlineMs,
    depth: 0,
  });
}
