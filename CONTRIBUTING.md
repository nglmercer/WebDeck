# Contributing to WebDeck

Start with the [source quickstart](README.md), [architecture](docs/architecture.md), and [implementation roadmap](docs/improvement-plan.md). Current evidence and unfinished work are tracked in [improvement progress](docs/improvement-progress.md).

## Development setup

Use the Rust toolchain and native libraries described in README, plus Node 22.18 or newer. Install both JavaScript tool directories:

```sh
npm ci --prefix frontend
npm ci --prefix tools/component-check
npm run build --prefix frontend
npm run check:budget --prefix frontend
cargo build --locked --bin webdeck
cargo run --locked --bin webdeck -- --no-tray
```

The server serves the production frontend from disk. Rebuild the frontend after UI changes. For an isolated browser fixture, use the acceptance launcher; it creates and removes its own configuration directory and enables fake effects. Do not run native-effect tests against a personal deck without choosing the actions deliberately.

## Where changes belong

| Responsibility | Owner |
| --- | --- |
| Command/configuration shape | `contracts/v2.schema.json`; generated Rust/TypeScript contracts |
| Schema defaults and UI field interpretation | `frontend/src/lib/schema.ts` |
| HTTP, credentials, realtime correlation | `frontend/src/lib/api/` |
| Boot generations, configuration loading | `frontend/src/lib/session.svelte.ts` |
| Asset fetching and object URL disposal | `frontend/src/lib/assets.ts` |
| Draft mutations, save generations, conflicts, undo | `frontend/src/features/editor/editor.svelte.ts` |
| Staged button form | `frontend/src/features/editor/button-draft.svelte.ts` |
| Deck placement, gestures, polling, command feedback | `frontend/src/features/deck/` |
| Settings and pairing presentation | `frontend/src/features/settings/`, `features/pairing/` |
| Interface language | `frontend/src/lib/messages.ts`, translations helpers, `webdeck/translations/*.lang` |
| Shared visual tokens | `frontend/src/styles/tokens.css` |
| Server composition and route policy | `src/server.rs`, `src/server/` |
| Accepted execution ownership and admission | `src/executor.rs` |
| Native command families and resource cleanup | `src/native.rs`, `src/native/` |
| Atomic persistence, grants, portable rollback | `src/storage.rs`, `src/sessions.rs`, `src/update.rs` |

Keep App as composition. Components receive typed data and callbacks; avoid direct competing draft mutations. Stage button-dialog edits until Apply, then use the editor owner. Preserve unknown extensions and configured IDs. Move generated files only by updating the generator, imports, and checks together.

For protocol changes, edit the canonical schema and run `node tools/contracts/generate.mjs`. Never hand-edit generated contracts. UI labels and controls supplement the schema; they must not introduce a second command definition.

Add stable translation keys to the English fallback and English resource together. Other locales may fall back to English; preserve placeholders. Run the translation guard after changing interface text. Use component styles for feature-specific rules and semantic tokens for shared values. Keep documented theme override points intact.

## Verification

Run focused behavior tests as you work, then the applicable complete checks before submitting:

```sh
node tools/contracts/generate.mjs --check
node tools/validation/v2-only.mjs
cargo fmt --all -- --check
cargo test --locked --all-targets
cargo clippy --locked --all-targets --all-features -- -D warnings
npm run format:check --prefix frontend
npm run check:i18n --prefix frontend
npm run check:components --prefix frontend
npm run typecheck --prefix frontend
npm run knip --prefix frontend
npm test --prefix frontend
npm run build --prefix frontend
cargo build --locked --bin webdeck
npm run check:budget --prefix frontend
npm run test:acceptance --prefix frontend
node tools/validation/diagnostics.mjs
# Optional verification report (contains no raw diagnostic payloads):
WEBDECK_DIAGNOSTICS_OUTPUT=docs/v2/evidence/improvements/diagnostics.json node tools/validation/diagnostics.mjs
```

Install Chromium with `npx playwright install chromium` from `frontend` if needed. Acceptance uses a rebuilt server and production UI. Fake-effect tests prove authorization and interface behavior; they do not prove desktop input, audio playback, OBS, or Spotify connectivity. Cross-target compilation likewise does not prove live Windows behavior.

For changes affecting startup, persistence, packaging, or the updater, build a development portable archive with `cargo run --locked --bin package -- --dev`, then run `node tools/validation/portable.mjs <archive>`. Set `WEBDECK_PORTABLE_ARTIFACT` to that archive and run `cargo test --locked --test v2_update portable_archive -- --ignored` for the real upgrade/rollback rehearsal. Archive paths follow the filename emitted by the package command.

CI checks production bundle ceilings with `npm run check:budget --prefix frontend`. Run `WEBDECK_PERFORMANCE_OUTPUT=<report.json> node tools/validation/performance.mjs` for comparable 48/1,024-button browser workloads, shared-icon request counts, polling observation, and keyboard enter/leave editing responsiveness. Keyboard timings include automation and readiness-observation overhead. Check that report locally with `node tools/validation/performance-budget.mjs <report.json>`. Timing budgets apply to the recorded machine/profile and are not CI timing guarantees; update budgets only with explained measurements. The profiler prints its report and writes it only when an output path is supplied.

Use `node tools/validation/grid-placement.mjs` for placement profiling; `WEBDECK_GRID_OUTPUT` optionally saves the report. Use `WEBDECK_PERFORMANCE_OUTPUT` and `WEBDECK_PORTABLE_OUTPUT` to preserve historical reports when recording new measurements. Separate algorithm timings, browser rendering, fake effects, and live desktop latency in claims. Reproduce the same workload and build before comparing results.

## Execution safety and diagnostics

Keep authorization central and test routes as well as helpers. Supplied device credentials never inherit loopback administrator access; configuration/source and integration secrets remain local-only. Check nested capabilities and budgets. A disconnected observer does not cancel accepted execution, and uncertain outcomes must never trigger automatic replay.

Tracked children and audio belong to explicit owners. Handle timeout and failure cleanup as well as shutdown. Input chords retain their own lock; unrelated actions must remain independent. Bound blocking helpers rather than introducing a global action lock.

Desktop `Open` and tray browser handoffs use a separate launcher waiter, with at most 64 outstanding direct launchers. Completion confirms successful spawning; it does not prove that the destination application opened the target. Each waiter reaps its direct child when it exits. These handoffs remain outside native-owner shutdown so a user's browser or document application survives quitting WebDeck. Explicitly managed process commands retain their own shutdown cleanup.

Configuration, boot, translations, asset, button resolution, and device persistence routes use a four-slot per-server blocking disk admission pool. Metrics and audio enumeration use a separate two-slot query pool. Check authorization before admission, and move the permit into the task so cancellation of an HTTP observer does not release capacity early. Saturation returns `capacity_exhausted`; it does not queue unbounded work or retry writes. Both pools are separate from native command admission. Blocking OS calls retain their slot until completion; admission limits cannot interrupt a stalled OS call.

Set `WEBDECK_DIAGNOSTICS=1` to enqueue a JSON terminal event for each command reaching the executor. A dedicated writer sends records to stderr through a 256-record queue; producers never wait for queue capacity or stderr. Events are best effort: saturation, writer startup failure, or process exit can drop records. The writer is not part of command admission or shutdown draining. Events contain the SHA-256 fingerprint of the caller's request ID, capability category, elapsed milliseconds, outcome, and optional canonical error code. Hash the client request ID locally to correlate an event; the raw ID is deliberately absent because IDs are caller-controlled. Logs do not include command arguments, headers, tokens, source, outputs, or free-form errors. Authentication failures rejected before the executor produce no executor event. Accepted task diagnostics continue after observer cancellation; a panic or interruption before a known terminal result records `unknown`.

Keep diagnostic fields allowlisted. Do not expand them by serializing a request or an error. Diagnostic output is opt-in; capture and retain it according to your local operational needs.

## Review evidence

Describe the concrete before/after behavior, ownership changes, and checks run. Add focused behavioral regressions for confirmed defects; avoid tests that merely mirror an extraction. For UI changes, inspect narrow/wide views and keyboard/touch flows, including conflict/offline states where relevant. Record unavailable platform/account checks explicitly in the status guide. Do not equate green automated checks with complete live platform verification.

Authorization and network-policy snapshots use a separate four-slot blocking pool shared by HTTP and realtime. Fresh grant-file reads preserve revocation across session owners; pool saturation fails closed before effect admission. Device listing uses the disk pool, and realtime usage shares the two-slot hardware-query pool.

For a frontend-only baseline comparison, build the original frontend in a separate directory and place its commit ID in `comparison-origin.txt` at that directory's root. Run `WEBDECK_FRONTEND_BASELINE=<directory> WEBDECK_PERFORMANCE_OUTPUT=<baseline.json> node tools/validation/performance.mjs`. The profiler uses the current backend with that directory's `frontend/dist`; baseline mode measures boot, shared-icon loading and bundle size, omitting modern editor/polling workflows. Compare matching workloads against a fresh current run. This is not historical whole-application timing. Run `node tools/validation/asset-loading.mjs` for a controlled baseline traversal/current-cache comparison; `WEBDECK_ASSET_OUTPUT` saves its report.
