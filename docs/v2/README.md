# WebDeck v2 — Full Refactor Proposal

Status: historical design, implemented on `v2`; see [implementation status](STATUS.md) for verification and outstanding release gates.
Target branch: `v2`
Reviewed base: `master` at `343baf22bccbcb26c6114a5795d1449c2d83a777`
Prepared: September 29, 2026

## Decision

Create `v2` as a non-release integration branch. Keep `master` as the v1 maintenance line until the v2 release gates are met. Refactor WebDeck into explicit domain, application, infrastructure, transport, and desktop boundaries while retaining Rust, Axum, Svelte 5, TypeScript, Vite, and Socket.IO.

This is a full architectural refactor delivered in small, reviewable increments—not a simultaneous rewrite of every file, a framework replacement, or a promise that a directory move alone improves the product. Every migrated feature must have a tested replacement and an explicit compatibility decision before its old implementation is removed.

The initial proposal described the design only. The `v2` implementation now changes runtime code and configuration migration; version numbers and release feeds remain unchanged. The sections below retain the original design rationale.

## 1. Findings from the reviewed revision

The current repository is already the Rust port, not the original Python application. Its root package is version `1.8.7`, with a shared library and five binaries. The frontend already depends on Svelte 5 and TypeScript. Replacing this stack would add migration scope without addressing the concrete problems identified here. [S1][S1][S2][S2]

The existing `docs/refactor.md` describes eight completed cleanup phases: shared modal behavior, a consolidated HTTP client, Svelte settings work, a smaller query helper, dependency cleanup, and CI gates. V2 should build on those changes rather than repeat them. Its historical test counts and success claims have not been independently reproduced during this review. [S3][S3]

| Area | Observed baseline | V2 objective |
| --- | --- | --- |
| Commands | `handle_command` dispatches by string prefixes and includes parsing, platform branches, logging, and effects. | Separate compatibility parsing, authorization, scheduling, and execution. |
| Configuration | `AppState` contains the folder queue and local IP; handlers reread configuration from disk. Atomic file replacement already exists. | Establish one configuration owner, typed validation, conflict handling, and controlled external-edit reloads. |
| Frontend | `installGlobals()` still exposes functions on `window`; wiring code reads and mutates DOM state through the custom query helper. | Move feature ownership and events into Svelte components and typed state. |
| Realtime | HTTP returns command results; Socket.IO emits the original message as `json_data`. Both command paths already use `spawn_blocking`. | Share execution internally without silently changing transport contracts; bound concurrent work. |
| Validation | CI runs Rust and frontend checks on Ubuntu; pushes target only `master`. The default Playwright configuration records a demo. | Run CI on `v2`, add Windows coverage, and add an assertion-based acceptance suite. |

Evidence for these observations: [S4][S4][S5][S5][S6][S6][S7][S7][S8][S8][S9][S9][S10][S10].

Documentation also needs reconciliation. For example, the build guide describes `typecheck` as Svelte-aware checking, while the actual package script is `tsc --noEmit`; architecture text references modules that the cleanup report says were deleted. Treat executable configuration and reviewed code as the source of truth, and update the affected guides in the same PR as future changes. [S2][S2][S3][S3][S11][S11][S12][S12]

## 2. Scope and non-goals

The refactor covers command dispatch, configuration, frontend state and UI wiring, HTTP and Socket.IO boundaries, integrations, native platform adapters, startup and shutdown, plugin execution, packaging, updater behavior, and regression testing.

Preserve the working browser macro-deck experience: folders, layouts, images, themes, language selection, editor behavior, usage tiles, audio, OBS, Spotify, scripts, Rhai plugins, tray/QR flows, and portable distribution. Inventory these against code and fixtures rather than treating documentation as proof of parity. [S13][S13]

Do not add cloud hosting, replace Socket.IO with raw WebSockets, introduce a database or external broker, switch frontend frameworks, or redesign the UI as a prerequisite. Do not delete apparently unused static assets: existing configurations can reference their filenames. Do not upgrade all dependencies while restructuring the application. [S3][S3]

Security changes that alter who may issue commands are intentional product changes, not behavior-neutral cleanup. Schedule them explicitly, test them separately, and document their effect on existing clients.

## 3. Target architecture

Start by enforcing module boundaries in the existing Rust package. Extract a small `webdeck-core` workspace member only after doing so produces a concrete benefit such as running parser/configuration tests without native GUI or audio dependencies. Do not create a crate for every integration. Cargo can retain the current root package while adding workspace members. [E1][E1]

```text
src/
  domain/                 # Command/config models, validation, errors, ports
  application/            # Command executor, config service, session policy
  adapters/
    config/               # Filesystem persistence and migrations
    platform/             # Windows/Linux input, windows, audio, power
    integrations/         # OBS, Spotify, soundboard, scripts, Rhai
  server/
    http/                 # Existing endpoints and separate v2 contracts
    socket/               # Existing events and separately versioned events
    middleware/           # Network, session, origin and request-size policy
  desktop/                # Startup, tray, QR, shutdown orchestration
  updater/                # Check, stage, validate, apply, rollback
  main.rs                 # Thin composition root
  bin/                    # Preserve existing entry points during extraction
frontend/src/
  app/                    # Boot and top-level composition
  features/
    deck/
    editor/
    settings/
    integrations/
  lib/
    api/                  # Existing centralized client plus typed contracts
    state/                # Shared state with explicit ownership and cleanup
    ui/                   # Reusable, accessible Svelte components
contracts/                # Schemas and transport compatibility fixtures
tests/                   # Unit, contract, migration and system checks
```

The tree is a proposed destination, not a requirement to move everything in one commit. Keep existing `src/app/` modules as temporary forwarding adapters while migrating features. Preserve `webdeck/`, `static/`, `frontend/dist/`, and runtime `.config/` paths until an explicit migration handles them. [S12][S12]

Dependency direction: domain depends on neither HTTP nor OS APIs; application depends on domain and small interfaces; adapters implement those interfaces; HTTP/Socket.IO and desktop callers invoke application services. Startup constructs concrete adapters and passes them to the callers. Filesystem, platform, and integration effects belong in adapters; HTTP/socket emission stays in transport, and lifecycle orchestration stays in the composition root.

## 4. Command execution design

Introduce a pure parser that produces typed commands, with a separate legacy parser/facade for existing command strings. Do not replace prefix matching with a naive whitespace split: match precedence, aliases, placeholders, escaping, empty arguments, and plugin fallback are part of compatibility. Pin the existing `/kill`, `/screensaver`, clipboard, soundboard, and overlapping-prefix behavior before altering dispatch. [S3][S3][S4][S4]

The intended flow is:

```text
HTTP / Socket.IO / desktop action
  -> validate request and identity
  -> parse legacy or versioned command
  -> check capabilities
  -> submit to a bounded executor
  -> invoke one platform/integration adapter
  -> map result to the caller's existing or v2 response contract
```

Use an explicit built-in command registry with stable names and documented legacy precedence. A command descriptor should cover argument shape, required capability, platform availability, and execution category. The UI catalog and executor should derive from a common definition where practical; plugin extension data must remain representable.

Separate independent integration I/O from ordered input/clipboard operations. Add admission limits and per-resource sequencing where concurrent actions would interfere. Distinguish a task being accepted from its side effect completing. Do not automatically retry a non-idempotent action such as typing text or triggering a macro after reconnect.

Preserve the existing `spawn_blocking` isolation for suitable synchronous calls, but add bounded admission and lifecycle tracking. A timeout on the awaiting request does not stop an already-running blocking closure; true cancellation needs cooperation or a managed subprocess. Long-lived native loops need dedicated ownership rather than occupying the general blocking pool indefinitely. [S8][S8][E2][E2]

Never validate the parser by executing destructive commands. Use fake platform adapters and recorded execution plans. Test `/exit` and similar process-lifecycle paths in a disposable child process.

## 5. Configuration and migration

Introduce `ConfigService` as the authoritative owner of a validated in-memory snapshot. Centralize all HTTP, socket, tray, plugin, and startup reads/writes through it. Keep serialization and migrations separate from business operations.

Retain atomic disk replacement. Add serialized read-modify-write transactions and revision checks for the new editor/API so two clients cannot silently overwrite one another's changes. A failed save must leave both the last valid snapshot and the last valid file intact. Do not hold a configuration lock while awaiting unrelated network or native operations.

Caching must not silently remove support for external file edits. Define a checked reload path using a file fingerprint or watcher, revalidate before publication, and detect edits that conflict with pending saves. Preserve folder insertion order, unknown extension keys, custom command strings, upload references, and plugin data. Existing grid resizing depends on folder order. [S6][S6]

Introduce an explicit schema version and deterministic migrations. Before migrating, create a recoverable backup with protected access; migrate a copy, validate it, then publish it atomically. A second migration of the same fixture must be a no-op. Define behavior for invalid JSON, unsupported future schemas, full disks, interrupted writes, and failed rollback.

Run tests and v2 trials with an isolated `WEBDECK_CONFIG_DIR`. Do not point v1 and v2 at the same mutable configuration. Legacy full-replacement endpoints may retain their documented last-write-wins behavior; new revision-aware endpoints should report a conflict rather than pretending the legacy caller supplied a revision. [S6][S6]

## 6. HTTP, realtime, and security boundaries

Keep existing endpoints and event names behind compatibility adapters until their removal is separately approved. Contract tests must cover status codes, response bodies, event recipients, ordering where guaranteed, and error behavior. In particular, do not change `json_data` from an original-message echo to a command-result object in place. [S5][S5][S8][S8]

For genuinely new structured commands, add an explicitly versioned API/event contract, for example `/api/v2/commands`, rather than silently changing `/send-data`. Use one schema source for new transport DTOs and a generated TypeScript representation or equivalent drift check. Keep transport DTOs distinct from internal models and user configuration.

Carry a request/command identifier through execution. New versioned contracts should distinguish invalid input, unsupported platform/capability, authorization failure, unavailable integration, capacity exhaustion, and execution failure. Keep client errors sanitized and logs useful without recording credentials or sensitive command arguments.

Retain the LAN/CIDR guard, but design authorization for command execution separately. Proposed v2 security work includes locally approved device pairing, revocation, consistent HTTP and Socket.IO enforcement, explicit permissions for scripts/power/plugin actions, origin/CSRF protections appropriate to the chosen session transport, and request/upload limits. These are proposed changes, not findings from a complete security audit. The reviewed router currently disables Axum's default body limit. [S5][S5]

Review upload path containment, plugin execution budgets, native process launching, and updater artifact verification as part of that work. Do not expose the service beyond its intended local network as part of this refactor. Document any TLS or trusted-reverse-proxy assumptions instead of treating a pairing token over plain HTTP as complete transport security.

## 7. Frontend ownership and lifecycle

Keep the existing centralized HTTP client and incremental Svelte 5 work. Migrate one feature at a time: deck navigation and submission, editor drafts and save flows, settings, then integration controls. [S2][S2][S3][S3]

Replace inline/global handlers with typed Svelte callback props and component-owned events. Svelte 5 supports event-handler properties and callback props; this is a way to remove global wiring, not a reason to introduce another framework. [S7][S7][E3][E3]

Represent the active folder, editor draft, pending saves, selected modal, transport state, and usage subscription explicitly. Keep an unsaved draft separate from persisted configuration. Ensure subscriptions, timers, sockets, event listeners, and object URLs are cleaned up on unmount or replacement. Make refresh/reconnect tests prove that one click causes exactly one submission.

Keep theme/CSS hooks and mobile interactions compatible. Add tests for keyboard navigation, modal focus restoration, touch actions, disabled pending controls, validation messages, and error recovery. Remove `window` bridges and the custom query helper only when no production call sites remain and migrated features have acceptance coverage.

## 8. Integrations, native platforms, and distribution

Introduce small interfaces around OBS, Spotify, audio, clipboard/input, window management, system actions, filesystem access, and plugins. Use deterministic fakes in default tests. Cover live integrations with explicitly enabled tests and isolated credentials/devices; never execute power, input, or user scripts on a shared CI runner.

Keep platform behavior explicit. Add Windows build/test coverage and Linux desktop smoke checks; a mocked unit test is not proof that COM, tray, audio, Wayland, or display-dependent paths work. The earlier cleanup report explicitly left native validation as follow-up work. [S3][S3]

Preserve binary names, CLI flags, portable ZIP layout, startup ordering, and updater handoff until their respective tests pass. The current package includes application, updater and QR binaries plus static/default/frontend assets. Add packaged-artifact tests that launch a safe console configuration, serve the built UI, and verify isolated config survival through an update/rollback rehearsal. [S11][S11]

Treat dependency upgrades and release-channel changes as separate PRs. Do not publish `v2` builds to the stable updater channel automatically. Decide one release-version source before the first prerelease; do not merely change `Cargo.toml` to `2.0.0` while other version/catalog metadata remains inconsistent.

## 9. Rollout and completion criteria

Use `refactor/v2-*` task branches targeting `v2`. Require green checks and a reviewed compatibility note for each slice. Keep fixes flowing from `master` into `v2` deliberately; do not force-push either branch or change the default branch during the refactor.

The ordered PR plan and acceptance gates are in [IMPLEMENTATION_PLAN.md](WebDeck-v2-implementation-plan.md). V2 is ready for release only when:

- All preserved features have parity evidence or an explicitly approved migration/deprecation; no core command path relies on unreviewed legacy fallback.
- Configuration migration, external-edit handling, interrupted-save recovery and rollback are proven with isolated fixtures and packaged builds.
- Windows and Linux validation, assertion-based browser tests, existing Rust/frontend gates and packaging smoke tests pass without unexplained unhandled errors.
- HTTP/Socket.IO compatibility and intentional authentication/API changes are documented and tested; sensitive operations have bounded execution and appropriate authorization.
- Performance comparisons use the same recorded workloads and environments; there is no accepted regression without an explanation and owner.
- Documentation, updater/release metadata and user-facing migration instructions match the executable build.

No latency, memory, coverage percentage, or delivery-time improvement is claimed before a measured baseline exists.

## 10. Review and execution limits

This proposal is based on targeted source/documentation inspection at the pinned commit, not a complete line-by-line audit. Application builds, Rust/frontend tests, live integrations and native platform behavior were not executed here. The starter patch is validated for syntax, whitespace and application against a reconstructed copy of the exact CI file it changes; that does not establish application correctness.

At preparation time, GitHub rejected this session's attempt to create `v2` with HTTP 403, `Resource not accessible by integration`. No remote branch, commit or pull request was created by this session. Direct cloning was also unavailable in this execution environment, so the deliverable is an apply-ready patch rather than a local branch bundle.

## Sources

[S1]: https://github.com/nglmercer/WebDeck/blob/343baf22bccbcb26c6114a5795d1449c2d83a777/Cargo.toml
[S2]: https://github.com/nglmercer/WebDeck/blob/343baf22bccbcb26c6114a5795d1449c2d83a777/frontend/package.json
[S3]: https://github.com/nglmercer/WebDeck/blob/343baf22bccbcb26c6114a5795d1449c2d83a777/docs/refactor.md
[S4]: https://github.com/nglmercer/WebDeck/blob/343baf22bccbcb26c6114a5795d1449c2d83a777/src/app/buttons/commands/mod.rs
[S5]: https://github.com/nglmercer/WebDeck/blob/343baf22bccbcb26c6114a5795d1449c2d83a777/src/app/server/mod.rs
[S6]: https://github.com/nglmercer/WebDeck/blob/343baf22bccbcb26c6114a5795d1449c2d83a777/docs/state-config.md
[S7]: https://github.com/nglmercer/WebDeck/blob/343baf22bccbcb26c6114a5795d1449c2d83a777/frontend/src/app/wireup.ts
[S8]: https://github.com/nglmercer/WebDeck/blob/343baf22bccbcb26c6114a5795d1449c2d83a777/src/app/server/realtime.rs
[S9]: https://github.com/nglmercer/WebDeck/blob/343baf22bccbcb26c6114a5795d1449c2d83a777/.github/workflows/ci.yml
[S10]: https://github.com/nglmercer/WebDeck/blob/343baf22bccbcb26c6114a5795d1449c2d83a777/frontend/playwright.config.ts
[S11]: https://github.com/nglmercer/WebDeck/blob/343baf22bccbcb26c6114a5795d1449c2d83a777/docs/build-release.md
[S12]: https://github.com/nglmercer/WebDeck/blob/343baf22bccbcb26c6114a5795d1449c2d83a777/docs/architecture.md
[S13]: https://github.com/nglmercer/WebDeck/blob/343baf22bccbcb26c6114a5795d1449c2d83a777/README.md
[E1]: https://doc.rust-lang.org/cargo/reference/workspaces.html
[E2]: https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html
[E3]: https://svelte.dev/docs/svelte/v5-migration-guide
