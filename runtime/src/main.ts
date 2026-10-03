import { usage } from "webdeck:metrics";
import type { Capability, Command } from "./generated/contracts";

type ExecutionContext = Readonly<{
  capabilities: readonly Capability[];
  deadlineMs: number;
  depth: number;
}>;
type Definition = {
  capability: Capability;
  execute(command: Command): unknown;
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

const app = Object.freeze({
  invoke(command: Command, context: ExecutionContext): unknown {
    const definition = commands.get(command.type);
    if (!definition) throw new Error("Command has not migrated");
    if (!context.capabilities.includes(definition.capability)) {
      throw new Error("Capability denied");
    }
    return definition.execute(command);
  },
});
(globalThis as unknown as { __webdeckRuntime: typeof app }).__webdeckRuntime = app;
