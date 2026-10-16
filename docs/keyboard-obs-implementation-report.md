# WebDeck Keyboard + OBS Automation — Implementation Report

## Summary

Implementation commit: `401e817` (local `master`; no PR or push).

Baseline: clean `master`, HEAD `e5da063`. Implementation stays within Rust,
Svelte, canonical contracts, the embedded runtime and `webdeckctl`.

Completed: shared keyboard catalog and validation; focused shortcut picker;
F1–F24, numpad digits and mapped lock/system keys; release cleanup; authenticated
capability discovery; corrected named hotkeys and separate key sequences with
modifiers; a shared bounded OBS connection; local-admin resource requests;
versioned project schema/loader; pure reconciliation; UUID bindings and rename
handling; offline init/validate/schema; diff/dry-run/apply; managed pruning;
nested overlays; explicit managed instance-property templates; scene-button
sync; examples, agent instructions and recovery documentation.

Partial scope: connection reuse serializes requests through a bounded queue;
there is no concurrent in-flight multiplexing, proactive reconnect/backoff,
event subscription cache or RequestBatch interface. A disconnected connection
is discarded and a later request reconnects; mutations are never automatically
replayed. Existing collections must be explicitly selected before discovery.
Local asset upload/bindings, collection replacement, canvas selection, optional
key sequences and MCP tools are not implemented. Unsupported project settings
and replacement mode produce errors/conflicts; no rollback is promised.

## Changed areas

- Rust keyboard/backend: `src/keys.rs`, `src/capabilities/desktop_input.rs`,
  `input.rs`, `primitives.rs`, `src/domain.rs`, capability endpoint.
- Svelte editor: `KeySelector.svelte`, shared key helpers, component/unit tests,
  `ActionFields.svelte`, English translation keys and browser acceptance test.
- Schema/contracts: `contracts/keys.json`, `v2.schema.json`, generated Rust/TS
  and catalog, plus `obs-project.schema.json`.
- OBS runtime/transport: `src/obs.rs`, trusted host primitive, scoped fake,
  runtime built-in source and rebuilt embedded JavaScript.
- Project engine: `src/admin/obs_project/` loader, planner, synchronizer,
  managed pruning, instance templates and pure tests.
- CLI/admin: typed flags, local-admin API, endpoint pinning, revision-aware
  scene-button provisioning and tests against a real loopback WebSocket mock.
- Examples/docs: `examples/obs-project`, project/LLM guides, CLI/automation
  references, README and manual acceptance matrix.

## Verification

Baseline passed: 61 Rust library tests and 73 frontend tests.
Automated validation includes contracts generation/check, Rust formatting,
all-target tests, Clippy with warnings denied, runtime typecheck, frontend
component/type/unit/i18n/build checks, browser acceptance, structure validation
and offline CLI JSON round trips. Final results: 158 Rust tests passed, two
existing artifact-dependent tests ignored; 77 frontend unit tests passed; browser
acceptance had 80 passed and one existing mobile-profile-only skip. The final
project-domain run had 73 library plus 32 admin tests passing; the async shutdown
lifecycle regression also passed separately.

Tests cover fail-fast chords, reverse release after a failed press, F mappings,
legacy editor strings, focused capture/cancellation, named versus sequence
hotkeys, unique response IDs, concurrent queued requests, disconnect recovery,
project cycles/paths/schema errors, no-write dry-run, idempotency, UUID renames,
managed ownership, unmanaged references, template overrides, custom labels,
button pruning, remote denial, endpoint changes and partial apply/retry.

Two frontend-serving tests initially failed after schema edits because the
built frontend still had the old contract fingerprint; rebuilding it resolved
those failures. An expanded transport test initially closed its mock connection
after two requests despite adding two more concurrent requests; its fixture was
corrected and the targeted test passed. No assertions or tests were disabled.
Two existing Rust integration tests require packaged/trusted artifacts; a
browser acceptance case is skipped outside mobile device profiles.

Physical Windows/X11/Wayland, portal denial and real OBS tests were not run.
The [manual matrix](keyboard-obs-manual-tests.md) marks every such row explicitly.
No real OBS mutation was sent to the existing user environment.

## Compatibility and security

Existing v2 `key`/OBS actions and add-only scene-button provisioning retain their
shape. Opening a key editor does not normalize or save it. Named `hotkey` now
uses `TriggerHotkeyByName`; saved raw `OBS_KEY_*` actions should migrate to
`hotkey_sequence`. Optional `context_name`, `key_modifiers` and `scene_uuid`
fields are additive.

Projects cannot supply integration credentials. Secrets remain in protected host
settings and are not sent to the frontend/runtime built-in. Generic plugins
cannot invoke the scoped OBS primitive; management requires local-admin/network
authority. Project files are read only in the CLI's local filesystem context.
OBS operations are bounded and nontransactional; endpoint/collection checks,
UUID ownership, explicit adoption, deletion opt-ins and journals reduce risk.
Deck writes retain revision conflict handling and idempotency.

Fn remains unavailable. Mapping availability is separate from desktop/portal
permission. Media controls use existing dedicated actions; menu and unmapped
consumer keys remain disabled.

## Quickstart

In the key editor, toggle Ctrl and Shift and choose F12, or focus **Record
shortcut** and press Ctrl+Shift+F12. Fn cannot be synthesized.

```sh
webdeckctl obs project init ./stream-project
webdeckctl obs project validate ./stream-project --json
webdeckctl obs project diff ./stream-project --json
webdeckctl obs project apply ./stream-project --dry-run --json
# Only after inspecting an explicitly designated collection:
webdeckctl obs project apply ./stream-project --yes --json
webdeckctl obs ensure-buttons --folder obs-scenes --sync --dry-run --json
webdeckctl obs ensure-buttons --folder obs-scenes --sync
```

For property propagation, see `instanceProperties` and `obs scene sync --all
--template FILE` in [the project guide](obs-project.md). After partial failure,
retain the binding state and journal, inspect OBS, re-run diff, resolve conflicts
and retry with the current deck revision. Do not blindly replay timed-out
mutations, remove the ownership file or assume changes were rolled back.
