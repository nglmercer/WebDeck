> Historical plan for the earlier compatibility implementation. Superseded by the v2-only implementation task; see [current status](STATUS.md). Do not use its compatibility requirements as current product requirements.

# WebDeck v2 — Implementation Plan

Status: original work breakdown; implementation and release-gate evidence are tracked in [STATUS.md](STATUS.md). Base: `343baf22bccbcb26c6114a5795d1449c2d83a777`.

Read the [architecture proposal](WebDeck-v2-proposal.md) first. Each task branch targets `v2`; do not merge unfinished v2 work directly into `master`. The branch names below avoid `v2/...` because a branch named `v2` already occupies that Git ref prefix once created.

## PR 0 — Proposal and branch CI

Initial commit on `v2`; alternatively use `refactor/v2-plan` targeting `v2` if repository rules require a pull request for this seed change.

The provided starter patch contains this phase's files only:

- `docs/v2/README.md`: architecture, compatibility, security, rollout and evidence.
- `docs/v2/IMPLEMENTATION_PLAN.md`: this acceptance-driven work plan.
- `.github/workflows/ci.yml`: change the push branch list from `[master]` to `[master, v2]`; retain the unrestricted pull-request trigger and all existing jobs.

Acceptance: patch applies cleanly at the reviewed base, no runtime/dependency/version changes, and the first pushed branch commit has visible CI results. The patch was prepared locally, not committed remotely.

## PR 1 — Reproducible baseline and behavioral inventory

Branch: `refactor/v2-baseline`.

Record the toolchain, OS, hardware, test commands and outputs at the baseline. Run the existing gates before changing behavior:

```sh
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets
(cd frontend && npm ci && npm run typecheck && npm run knip && npm run test && npm run build)
```

Audit what `typecheck` actually covers: it currently invokes TypeScript rather than the Svelte-aware checker described in part of the documentation. Add appropriate component checking in a separate reviewed change, without misreporting the existing script.

Create a parity matrix for command families, aliases, routes, socket events, editor flows, configuration variants, native platform behavior, plugins and packaging. Reproduce or resolve the Svelte teardown error mentioned in `docs/refactor.md`; do not suppress it globally just to make a run look green.

Measure startup-to-usable-UI, boot response latency, representative command latency, idle CPU/memory, repeated refresh behavior and frontend bundle size. Mark unmeasured items explicitly; agree on regression budgets using those observations.

Acceptance: baseline artifacts are reproducible; existing failures have a documented disposition; no historical test count is substituted for a current run.

## PR 2 — Safe test seams and transport contracts

Branch: `refactor/v2-contract-tests`.

Introduce narrow seams around native/process/network effects without rewriting command semantics. Use a fake executor in transport tests and fake platform adapters in command tests. Preserve or extend the existing CLI child-process tests.

Add golden/parameterized fixtures for command prefix precedence, aliases, placeholders, quoted paths, empty or malformed payloads, plugin fallback, and result/error shapes. Capture HTTP versus Socket.IO differences intentionally: an HTTP command result is not the existing `json_data` echo.

Build assertion-based Playwright tests separately from demo recording. Cover folder navigation, button creation/edit/deletion, grid resizing, theme/background changes, failed saves, reload persistence, reconnects and modal cleanup. At least one acceptance path should use the real Rust HTTP server with fake effects and an isolated config directory, not only mocked frontend fetches.

Acceptance: tests fail under a deliberate local regression in each critical path. No test touches the user's `.config`, starts an updater, executes scripts, shuts down a machine, or sends real input on a shared runner.

## PR 3 — Domain models and configuration owner

Branch: `refactor/v2-config-service`.

Add typed configuration, command and error models while preserving extension data. Introduce `ConfigService` behind existing callers first. Reuse atomic replacement; add serialized writes, revision-aware updates for new callers, and validation-before-publication.

Implement schema-versioned migrations, protected backups, external-edit reload policy, and clear recovery for invalid or unsupported config. Preserve folder insertion order and config-addressable asset paths. Keep v2 experiments isolated with `WEBDECK_CONFIG_DIR`.

Acceptance: old fixture -> migration -> serialization preserves all supported data; migrating twice is a no-op; concurrent edits yield a defined conflict or serialization outcome; interrupted or failed saves preserve the last valid configuration; rollback is exercised.

Do not require legacy clients to send an unavailable revision. Preserve and document the compatibility semantics of full-replacement legacy endpoints.

## PR 4 — Typed parser and bounded command executor

Branch: `refactor/v2-command-engine`.

Separate compatibility parsing from execution and route both transports through the same application service. Add typed built-in commands and a plugin extension path with explicit matching precedence. Move command families one at a time to platform/integration adapters.

Implement bounded admission, per-resource ordering where needed, meaningful accepted/completed states and log redaction. Keep suitable existing blocking isolation. Track subprocesses and long-lived workers; do not claim that dropping a timeout future cancels a running blocking operation.

Acceptance: all compatibility fixtures still pass; unknown or disallowed actions have specified behavior; overload is tested; repeated delivery does not accidentally produce silent automatic retries; teardown does not leak workers. Destructive branches are validated through fakes or isolated child-process tests.

## PR 5 — Versioned transport and explicit security changes

Branch: `refactor/v2-transport-security`.

Define versioned request/result/event schemas and a frontend drift check. Keep old routes/events as adapters instead of mutating their shape in place. Add stable error codes and request identifiers for the new contract.

Design device pairing, revocation and capability checks with local administrative approval. Apply equivalent policy to HTTP requests, Socket.IO handshakes and subsequent command messages. Specify reconnect/session-expiry behavior, origin/CSRF defense, body/upload limits, path confinement and plugin execution limits. Document transport-security assumptions.

Acceptance: allowed and denied cases are covered through both transports, including expired/revoked identities and reconnects; no privileged command bypasses the executor's policy; legacy compatibility versus intentional breaking changes is written down. This phase requires a focused security review rather than assuming LAN access alone is sufficient.

## PR 6 — Feature-owned Svelte UI

Branch: `refactor/v2-frontend-state` (split into deck/editor/settings PRs as needed).

Keep the consolidated HTTP layer. Move feature state, callback wiring and lifecycle cleanup into Svelte components. Separate editable drafts from persisted snapshots; expose conflicts and failures rather than overwriting or pretending a save succeeded.

Replace global bridges incrementally. Keep stable CSS/theme hooks and keyboard/touch behavior. Add tests for focused modal interactions, one submission per action, cancelled uploads, pending saves, refreshes, usage subscriptions and socket teardown.

Acceptance: assertion-based browser tests pass for migrated features; no duplicate handlers/sockets/timers or unexplained console errors appear after repeated navigation; remove a legacy helper only after all production call sites are gone.

## PR 7 — Integration/native isolation and platform CI

Branch: `refactor/v2-platform-adapters`.

Finish extracting OS-specific code and integration clients behind the tested interfaces. Centralize timeout, retry, token/config access and lifecycle ownership without indiscriminately retrying side-effecting operations.

Add Windows Rust checks alongside the existing Ubuntu jobs. Separate portable headless tests from GUI/audio/device-dependent smoke tests. Use explicit opt-in environments for live OBS/Spotify/audio/native integration tests. Define the supported OS/architecture matrix rather than advertising untested platform support.

Acceptance: Windows and Linux behavior has appropriate evidence; platform-specific code does not leak into domain tests; failures of one integration do not invalidate the whole command service; shutdown releases owned workers/resources.

## PR 8 — Distribution, updater, documentation and v2 readiness

Branch: `refactor/v2-release-readiness`.

Test the built portable artifact, not only source execution. Preserve application/updater/QR binary names and required default, static and frontend files. Rehearse upgrade and rollback against disposable copies of user configurations. Review archive/path safety and artifact authenticity/integrity before publishing through an updater.

Reconcile README and docs with the new code. Confirm release repository, channels, version metadata and prerelease naming explicitly; do not alter the stable update channel just because a branch is called `v2`.

Run performance comparisons against PR 1's recorded workloads. Remove old implementation paths only after the parity matrix is complete. A major-version release requires release notes for intentional changes and recovery instructions, not only a version-number bump.

Acceptance: all quality and compatibility gates pass, packaged smoke/upgrade/rollback tests succeed, supported-platform checks are recorded, and a maintainer approves stable cutover. Until then, retain `master` for v1 maintenance.

## Required evidence on each implementation PR

Use a small PR description with: scope; compatibility impact; tests and their actual results; migration/rollback impact; and known limitations. Separate mechanical moves from semantic changes where practical.

A proposed check is not a completed check. Report native/manual validation separately from automated tests, and do not count recorded demos or mocked adapters as proof that real integrations work.
