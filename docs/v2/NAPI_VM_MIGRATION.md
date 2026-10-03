# napi-vm migration: first milestone

Implementation branch: `codex/v2-napi-vm`.
V2 baseline: `a005d6244a050f62afe5b08e212a6dd39270c5e3`.
Pinned napi-vm revision: `881cc8f1cec262049b6d31fa499b0d601ba673c5`.
The dependency disables default features, including the Node N-API binding.

## Baseline verification

Before implementation, Rust formatting, Clippy with all targets/features and
warnings denied, and all-target tests passed (63 tests, one intentionally ignored
portable-archive test requiring a separately built artifact). Frontend clean install,
formatting, contract generation, translations, Svelte component checks, typecheck,
Knip, 51 unit tests, build and bundle budget passed. The V2-only guard passed.
All 56 browser acceptance tests passed. The baseline has no `npm run check`;
its component and typecheck scripts provide those checks.

## Implemented boundary

`HTTP / Socket.IO -> Executor -> VmAdapter -> VmRuntime owner thread -> embedded JS`

The adapter remains behind the existing Executor. A bounded 16-request queue sends
plain Rust commands and authorized contexts to one long-lived interpreter. Guest
values and the interpreter are created, used and dropped on that thread. Boot
failure aborts startup; there is no native fallback in VM mode. Shutdown joins the
owner after the Executor drains accepted roots and is idempotent.

JavaScript registers `debug` and `usage`. The `webdeck:metrics` module calls a
Rust capability, which checks the original context before and after native metrics
collection. The immutable guest context contains effective capabilities, depth and
an absolute deadline in Unix milliseconds; it contains no session tokens.
Instruction, loop and execution-time budgets bound guest execution. Queue waiting
uses the existing root deadline; expired queued commands cannot start host work.
Observation timeout does not release accepted ownership while a native host call
is still running. Native cancellation remains cooperative.

Contracts are generated from the existing schema for Rust, frontend and runtime.
Built-in JavaScript is compiled in development and embedded with `include_str!`;
WebDeck never starts Node, Bun or npm to load it.

## Development

```sh
npm ci --prefix runtime
npm --prefix runtime run check
npm --prefix runtime run build
cargo run --bin webdeck -- --no-tray --runtime vm
```

The default adapter remains native during migration. `--runtime vm` is restricted
to debug builds and supports only `debug` and `usage`; other commands fail clearly.
The metrics query endpoint remains in Rust. The transitional VM is not suitable
as the default for an existing deck yet. `WEBDECK_FAKE_EFFECTS` affects native
adapter selection; it does not bypass an explicitly selected VM.

Generated `runtime/dist/main.js` is committed, so Cargo builds do not require a JS
toolchain. CI rebuilds it and rejects drift. Node is a development build tool only.

## Parity and remaining work

| Command | Native | VM | Verification |
| --- | --- | --- | --- |
| debug | yes | yes | Golden tests with nested JSON and source-like strings |
| usage | yes | yes | Same fake primitive through VM; HTTP integration |
| remaining commands | yes | no | Subsequent milestones |

Regression tests cover real HTTP routing through the Executor, owner-thread reuse,
concurrent callers, capability denial at the Rust host boundary, expired/deep
contexts, error recovery, cooperative deadlines and drain/shutdown. They do not
require desktop effects. Linux is the locally exercised platform; Windows/macOS
runtime execution and a complete release test with Node/Bun absent remain pending.
The Linux development executable was smoke-tested with Node, Bun and npm absent
from `PATH`: VM boot, HTTP debug/usage and graceful shutdown passed. After the
change, all 72 Rust tests passed (one artifact-dependent test ignored), together
with formatting, Clippy, generated-contract checks and runtime TypeScript build.

Next milestones are network/fetch, input/clipboard, window/process, power and audio,
then scripts and plugins. Rhai, OBS, Spotify and native dispatch remain until their
respective parity is established. Workflow and plugin UI work has not started.
