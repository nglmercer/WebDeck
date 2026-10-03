# WebDeck v2 embedded runtime

Implementation branch: `codex/v2-napi-vm`. Baseline: `a005d6244a050f62afe5b08e212a6dd39270c5e3`.
The runtime and trusted plugin SDK/host are pinned to napi-vm revision
`881cc8f1cec262049b6d31fa499b0d601ba673c5`. The VM dependency disables its Node
binding/default features. WebDeck requires no Node, Bun or npm at runtime.

## Ownership

`HTTP / Socket.IO → Executor → VmAdapter → VM owner → JavaScript → authorized Rust primitive`

The VM is the sole production command adapter. Its owner thread constructs, uses
and drops all guest values. A bounded queue transports plain Rust data; boot failure
aborts startup. There is no native command dispatcher or fallback. Rust retains
contracts, authentication, grants, configuration, assets, OS/audio ownership,
HTTP/WebSocket transport, OAuth approval/state, the frontend server and shutdown.

`runtime/src/main.ts` owns the command registry. `runtime/src/core/` supplies
workflows and event subscriptions. Built-in OBS/Spotify JavaScript implements their
protocols using bounded native transport and scoped credentials. Built-in OBS,
Spotify, HTTP and soundboard packages also expose typed plugin actions. The command
catalog combines generated contracts with the active registry and verified plugin
manifests, filtering individual actions by the current caller's grant.

| Behavior | JavaScript owner | Rust boundary |
| --- | --- | --- |
| Debug, metrics requests | command registry | authorized native metrics |
| HTTP status policy | command registry | bounded HTTP response transport |
| Input, clipboard and key composition | command registry | atomic input transaction |
| Desktop, processes and power | command registry | constrained platform operations |
| Audio/media/soundboard | command registry | native devices, playback and cleanup |
| OBS actions and health | built-in OBS module | opaque WebSocket handles, scoped secrets, hashing |
| Spotify library/playback/playlist/follow/volume | built-in Spotify module | bounded HTTP, scoped token persistence |
| JavaScript scripts | isolated script interpreter | source confinement and inherited grants |
| JavaScript plugins | isolated persistent package interpreter | manifests, argument/result validation, scoped storage |
| Trusted native plugins | napi-vm plugin host | executable package integrity, IPC and process lifecycle |
| Workflows | workflow module | nested command authorization without new admission |

The legacy Rhai, obws and rspotify dependencies and Rust command dispatcher are
removed. Shell commands remain an explicit Script capability with bounded source,
output and timeout; arbitrary shell access is never a general host module.

## Scripts and packages

A script uses `language: "javascript"` and an inline or confined file source. For example:

```javascript
ctx.invoke({type: 'debug', data: {answer: 42}})
```

Each invocation gets fresh globals. `ctx` exposes invocation, deadline, a cooperative
abort signal, presentation events and redacted logging. No filesystem, process,
network or OS module is implicitly exposed. TypeScript is compiled during development,
not evaluated in the product. Earlier Rhai source requires manual translation;
WebDeck never rewrites user files or interprets Rhai as JavaScript.

Install a JavaScript directory package at `<config-dir>/plugins/<id>/`. Copy
[`examples/plugins/echo`](../../examples/plugins/echo) as a working example. Its
`webdeck.json` requires schema version, ID, semver version, backend, origin, entry,
SHA-256 of exact entry bytes, contract, and typed action definitions. Sandbox packages
use `backend: "sandbox_js"`, a `.js` entry and an empty contract. Export
`invoke_action(action, args, ctx)` and optional `onLoad(ctx)` / `onUnload(ctx)`.

Caller grants must include Plugin and every capability declared by the action.
Execution narrows to that declaration, and Rust rechecks the original authorized
context at every primitive. Package identity cannot be forged by changing guest
globals. Scoped storage in `webdeck:storage` persists at
`<config-dir>/plugin-state/<id>.json`; scripts have no independently assigned plugin
identity. General Network grants cannot read OBS or Spotify credentials.

Sandbox plugins retain their own globals between calls, receive a refreshed deadline
for each invocation, and can invoke another package without borrowing its globals.
Recursive invocation of an already-running package is rejected. Execution failures
and invalid results disable that package until explicitly enabled or reloaded;
ordinary argument validation failures do not disable it.

A trusted package uses `backend: "trusted_process"`, an entry pointing at its napi-vm
`plugin.json`, and the provided contract ID. Only an executable launch is accepted;
JavaScript process launches are rejected. The independent host verifies its production
`plugin.lock.json`, target, contracts and artifact hashes before starting the child.
Native processes are explicitly trusted and have ordinary OS privileges; manifest
capabilities govern WebDeck invocation, not an OS sandbox. The Rust SDK fixture is
[`examples/trusted-plugin.rs`](../../examples/trusted-plugin.rs).

Symlinks, parent traversal, absolute entries, reserved `builtin.*` IDs, duplicate action
IDs, invalid versions, unsupported formats and digest mismatches are rejected.
Startup quarantines invalid packages while loading valid ones. An explicit reload
validates all packages first and leaves the active registry unchanged on failure.

## Workflows and presentation

Both commands and button actions support workflows. Nodes include command, sequence,
parallel, conditional, delay, retry, timeout, variable, result reference, script and
plugin. `$result` objects and command `references` resolve earlier results/variables;
prototype-related keys and changes to the command type are rejected. Inputs are
validated again after resolution. Nested calls inherit the root's capabilities,
remaining deadline and admission slot.

Parallel nodes overlap guest promises and timers. One VM owner serializes synchronous
native calls and ordinary root execution; this implementation does not promise
parallel native effects. Timeout is cooperative and cannot undo an already-started
OS effect. Retry is explicit workflow behavior; clients never automatically replay
an uncertain command.

`ctx.emit({api_version:2,type:'button.stateChanged',button_id:'...',label:'...',active:true})`
updates connected deck presentation without editing configuration. Events are schema
validated, bounded, and forwarded only while the observer's current Read grant is
valid. Socket disconnection releases the subscription.

## Management and limits

Local Settings exposes package versions/backends, runtime health, reload and external
plugin enable/disable. `/api/v2/runtime`, `/api/v2/runtime/reload`, and
`PUT /api/v2/runtime/plugins/<id>` require local administration and Settings. Controllers
receive the effective catalog but cannot use these management routes. Enable/disable
is runtime state; restart/reload restores verified installed packages. Built-in
packages cannot be disabled through the external-package control.

The Executor owns up to 16 accepted roots; the VM queue also holds at most 16. Root
observation expires after 30 seconds but ownership lasts until native completion.
Nested depth is at most eight, guest call depth is bounded, and host callbacks are
limited to 1,024 per root. VM fuel and loops are bounded. Script/package sources,
HTTP bodies and scoped package state are limited to 64 KiB. WebSocket messages and
handles have native bounds and ownership checks. Workflow collections, retries,
delays, event fields and schema recursion are bounded. These are execution/structure
limits, not an OS memory sandbox for trusted native processes.

Reload runs bounded unload hooks, clears isolated interpreters, shuts down trusted
children and boots a new registry. Shutdown closes admission, drains accepted roots,
runs unload hooks and releases sockets, children and audio before joining the owner.
If failed root execution leaves queued guest work, further execution fails closed
until an explicit reload boots a clean runtime.

## Verification

The Linux verification suite covers Rust formatting/Clippy with warnings denied,
all-target tests, capability denial, independent script globals, persistent/narrowed
plugins, nested identities, events, storage, lifecycle/reload, workflows, OBS handshake,
Spotify requests, local HTTP parity, native plugin integrity/crash isolation,
frontend contracts/translations/types/components/unit tests/build/budget, browser
acceptance, redacted diagnostics and portable artifact checks.

`tools/validation/runtime.mjs` starts the real product with an empty tool PATH and
executes debug, scripts, plugins, workflows, catalog and reload operations. Native
plugin fixtures also run without an external JavaScript runtime. CI runs Linux,
Windows and macOS Rust checks; Windows/macOS execution was not verified locally.
Live desktop input/audio, destructive power actions, real OBS and Spotify credentials
remain opt-in checks and were not exercised by fake-effect acceptance tests.

The checked [validation report](evidence/napi-vm-validation.json) records 86 standard Rust
tests, both separately enabled fixture/archive tests, 51 frontend unit tests and 58
browser acceptance tests. The frontend remains within its existing bundle ceilings
(221,392 raw bytes / 66,415 gzip bytes).
