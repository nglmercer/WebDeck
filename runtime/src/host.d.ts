declare module "webdeck:metrics" {
  export function usage(): import("./generated/contracts").UsageSnapshot;
}

declare module "webdeck:network" {
  export function fetch(input: { method: string; url: string; headers: Record<string,string>; body: string; timeoutMs: number }): { status: number; body: string; truncated: boolean };
}

declare module "webdeck:host" {
  export function call(operation: string, input: unknown): any;
}

declare module "webdeck:scripts" { export function execute(source: string): unknown; }
