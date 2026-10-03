declare module "webdeck:metrics" {
  export function usage(): import("./generated/contracts").UsageSnapshot;
}
