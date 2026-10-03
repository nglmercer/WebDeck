import { execute as workflow } from "./core/workflows.js";
import { invoke } from "webdeck:commands";
import { execute as obs } from "./builtins/obs/index.js";
import { execute as spotify } from "./builtins/spotify/index.js";
import { invoke as invokeBuiltin } from "webdeck:builtins";
import { invoke as invokePlugin } from "webdeck:plugins";
import { execute as executeScript } from "webdeck:scripts";
import { call } from "webdeck:host";
import { fetch } from "webdeck:network";
import { usage } from "webdeck:metrics";
import type { Capability, Command } from "./generated/contracts";

type ExecutionContext = Readonly<{
  capabilities: readonly Capability[];
  deadlineMs: number;
  depth: number;
}>;
type Definition = {
  capability: Capability;
  execute(command: Command, context: ExecutionContext): unknown;
};
const commands = new Map<string, Definition>();
commands.set("debug", {
  capability: "read",
  execute(command) {
    if (command.type !== "debug") throw new Error("Invalid command");
    return { data: command.data };
  },
});
commands.set("usage", { capability: "read", execute: () => usage() });

commands.set("fetch", {
  capability: "network",
  execute(command) {
    if (command.type !== "fetch") throw new Error("Invalid command");
    const result = fetch({
      method: command.method,
      url: command.url,
      headers: command.headers,
      body: command.body,
      timeoutMs: Math.min(30000, command.timeout_seconds * 1000),
    });
    if (result.status < 200 || result.status >= 300)
      throw new Error("HTTP request failed");
    return {
      status: result.status,
      body: result.body,
      truncated: result.truncated,
    };
  },
});
const primitiveCommands: Record<
  string,
  { capability: Capability; execute(command: any): unknown }
> = {
  exit: { capability: "power", execute: () => call("system.exit", {}) },
  key: {
    capability: "input",
    execute: (c) =>
      call("input.perform", { steps: [{ kind: "press", keys: c.keys }] }),
  },
  write: {
    capability: "input",
    execute: (c) =>
      call("input.perform", {
        steps: [
          { kind: "text", text: c.text },
          ...(c.send ? [{ kind: "press", keys: ["enter"] }] : []),
        ],
      }),
  },
  copy: {
    capability: "input",
    execute: (c) =>
      call("input.perform", {
        steps: c.use_selection
          ? [{ kind: "press", keys: ["ctrl", "c"] }]
          : [{ kind: "clipboard", text: c.text }],
      }),
  },
  paste: {
    capability: "input",
    execute: (c) =>
      call("input.perform", {
        steps: [
          ...(c.use_selection ? [] : [{ kind: "clipboard", text: c.text }]),
          { kind: "press", keys: ["ctrl", "v"] },
        ],
      }),
  },
  cut: {
    capability: "input",
    execute: () =>
      call("input.perform", {
        steps: [{ kind: "press", keys: ["ctrl", "x"] }],
      }),
  },
  clipboard: {
    capability: "input",
    execute: () =>
      call("input.perform", {
        steps: [{ kind: "press", keys: ["meta", "v"] }],
      }),
  },
  clear_clipboard: {
    capability: "input",
    execute: () =>
      call("input.perform", { steps: [{ kind: "clipboard", text: "" }] }),
  },
  speech_recognition: {
    capability: "input",
    execute: () =>
      call("input.perform", {
        steps: [{ kind: "press", keys: ["meta", "h"] }],
      }),
  },
  open: {
    capability: "window",
    execute: (c) => call("window.open", { target: c.target }),
  },
  foreground: {
    capability: "window",
    execute: (c) => call("window.foreground", { target: c.target }),
  },
  close_focused: {
    capability: "window",
    execute: () => call("window.closeFocused", {}),
  },
  kill: {
    capability: "window",
    execute: (c) => call("window.kill", { target: c.target }),
  },
  restart: {
    capability: "window",
    execute: (c) => {
      call("window.kill", { target: c.target });
      return call("process.spawn", { executable: c.target, args: [] });
    },
  },
  restart_desktop: {
    capability: "window",
    execute: () => call("window.restartDesktop", {}),
  },
  color_picker: {
    capability: "window",
    execute: () => call("capture.pick", {}),
  },
  screensaver_settings: {
    capability: "power",
    execute: () => call("system.screensaverSettings", {}),
  },
  screensaver: {
    capability: "power",
    execute: (c) => call("system.screensaver", { mode: c.mode }),
  },
  firewall: { capability: "admin", execute: () => call("system.firewall", {}) },
  volume: {
    capability: "audio",
    execute: (c) => call("audio.volume", { change: c.change }),
  },
  app_volume: {
    capability: "audio",
    execute: (c) =>
      call("audio.appVolume", { application: c.application, change: c.change }),
  },
  microphone: {
    capability: "audio",
    execute: (c) => call("audio.endpoint", { device: c.device, input: true }),
  },
  speakers: {
    capability: "audio",
    execute: (c) => call("audio.endpoint", { device: c.device, input: false }),
  },
  play_sound: {
    capability: "audio",
    execute: (c) =>
      call("audio.play", {
        source: c.source,
        volume: c.volume,
        outputDevice: c.output_device,
        microphone: c.microphone,
        localOnly: c.local_only,
      }),
  },
  stop_sound: { capability: "audio", execute: () => call("audio.stopAll", {}) },
  shell: {
    capability: "script",
    execute: (c) =>
      call("process.shell", {
        source: call("storage.source", { source: c.source }),
        timeoutMs: Math.min(30000, c.timeout_seconds * 1000),
      }),
  },
};
for (const [id, definition] of Object.entries(primitiveCommands))
  commands.set(id, definition);
function powerCommand(verb: string): Definition {
  return { capability: "power", execute: () => call("system.power", { verb }) };
}
for (const [id, verb] of Object.entries({
  shutdown: "poweroff",
  reboot: "reboot",
  sleep: "suspend",
  hibernate: "hibernate",
  lock: "lock",
})) {
  commands.set(id, powerCommand(verb));
}
function mediaCommand(action: string): Definition {
  return {
    capability: "audio",
    execute: () => call("audio.media", { action }),
  };
}
for (const [id, action] of Object.entries({
  play_pause: "play-pause",
  previous: "previous",
  next: "next",
  mute: "mute",
})) {
  commands.set(id, mediaCommand(action));
}
commands.set("obs", {
  capability: "network",
  execute: (command) => invokeBuiltin(command),
});
commands.set("spotify", {
  capability: "network",
  execute: (command) => invokeBuiltin(command),
});
commands.set("button", {
  capability: "read",
  execute(command) {
    if (command.type !== "button") throw new Error("Invalid button");
    return invoke(call("storage.button", { id: command.button_id }));
  },
});
commands.set("workflow", {
  capability: "read",
  execute(command, context) {
    if (command.type !== "workflow") throw new Error("Invalid workflow");
    return workflow(command.workflow, context);
  },
});
commands.set("plugin", {
  capability: "plugin",
  execute(command) {
    return invokePlugin(command);
  },
});
commands.set("script", {
  capability: "script",
  execute(command) {
    if (command.type !== "script") throw new Error("Invalid script");
    return {
      value: executeScript(call("storage.source", { source: command.source })),
    };
  },
});
const app = Object.freeze({
  list() {
    return Array.from(commands.entries()).map(([id, definition]) => ({
      id,
      capability: definition.capability,
    }));
  },
  invoke(command: Command, context: ExecutionContext): unknown {
    const definition = commands.get(command.type);
    if (!definition) throw new Error("Command has not migrated");
    if (!context.capabilities.includes(definition.capability)) {
      throw new Error("Capability denied");
    }
    return definition.execute(command, context);
  },
});
(globalThis as unknown as { __webdeckRuntime: typeof app }).__webdeckRuntime =
  app;

(globalThis as any).__webdeckDispatch = app.invoke;

(globalThis as any).__webdeckBuiltinDispatch = (command: Command) => {
  if (command.type === "obs") return obs(command);
  if (command.type === "spotify") return spotify(command);
  throw new Error("Invalid builtin");
};
