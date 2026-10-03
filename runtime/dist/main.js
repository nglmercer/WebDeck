import { usage } from "webdeck:metrics";
const commands = new Map();
commands.set("debug", {
    capability: "read",
    execute(command) {
        if (command.type !== "debug")
            throw new Error("Invalid command");
        return { data: command.data };
    },
});
commands.set("usage", { capability: "read", execute: () => usage() });
const app = Object.freeze({
    invoke(command, context) {
        const definition = commands.get(command.type);
        if (!definition)
            throw new Error("Command has not migrated");
        if (!context.capabilities.includes(definition.capability)) {
            throw new Error("Capability denied");
        }
        return definition.execute(command);
    },
});
globalThis.__webdeckRuntime = app;
