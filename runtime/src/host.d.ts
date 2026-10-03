declare module "webdeck:metrics" {
  export function usage(): import("./generated/contracts").UsageSnapshot;
}

declare module "webdeck:network" {
  export function fetch(input: {
    method: string;
    url: string;
    headers: Record<string, string>;
    body: string;
    timeoutMs: number;
  }): { status: number; body: string; truncated: boolean };
}

declare module "webdeck:host" {
  export function call(operation: string, input: unknown): any;
}

declare module "webdeck:scripts" {
  export function execute(source: string): unknown;
}

declare module "webdeck:plugins" {
  export function invoke(
    command: import("./generated/contracts").Command,
  ): unknown;
}

declare module "webdeck:builtins" {
  export function invoke(
    command: import("./generated/contracts").Command,
  ): unknown;
}

declare function setTimeout(
  callback: (...args: any[]) => void,
  milliseconds: number,
): number;
declare module "webdeck:commands" {
  export function invoke(
    command: import("./generated/contracts").Command,
    timeoutMs?: number,
  ): unknown;
}
