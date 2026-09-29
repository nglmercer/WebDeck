# Cleanup & modernization refactor (Phases 1–8)

Goal: less code, fewer responsibilities, fewer duplicate implementations,
clearer state ownership — without changing functionality, UI, API contracts,
platform behavior, error handling, a11y, or lifecycle semantics. One commit
per phase, validated after each (`cargo fmt/clippy/test`, `tsc`, vitest,
vite build, Playwright demo tour).

## What changed

**Phase 1 — dead code, CSS, debug statements.** Removed unreachable
branches (`tempEditorConfig` null arm, `wireWakeLock`/`showInfo`), duplicate
CSS declarations, an `indexOf`-in-iteration, and stray `console.log`s.

**Phase 2 — Add/Edit modal sharing** (`frontend/src/views/`).
`button-upload.ts` (`uploadButtonImage`, `wireButtonImageUpload`) and the
`modalstyle.ts` guards (`wirePreviewControls`, `wireButtonNameSync`,
`wireDevCommandSync`, `beginModalSubmit`/`endModalSubmit`) hold the behavior
both modals shared. The artificial 1000 ms `setTimeout` saves are gone:
saves run immediately with the submit control disabled until the response
lands (no double-click duplicates).

**Phase 3 — one HTTP layer** (`frontend/src/api/`). `client.ts`
(`getJson`/`postJson`/`getText`/`postText`/`postForm`, `HttpError` with
timeout/abort) plus domain modules (`config`, `buttons`, `uploads`, `usage`)
replaced raw `fetch` at 15 call sites and the `query/ajax` helper.
Deleted `framework/api.ts` and `query/ajax.ts`.

**Phase 4 — Svelte 5 settings.** `ThemesPanel`/`BackgroundsPanel` own
bindable lists, `ColorField` self-syncs, `ArgsBlock` owns row-scoped
pickers/uploads (fixing a global-`input.filepath` bug),
`LoadingScreen` takes a `concealed` prop. Deleted `frontend/src/legacy/`
(7 files); `legacy/colors.ts` moved to `components/colors.ts`.

**Phase 5 — query framework trim.** Deleted `query/effects.ts`,
`query/data.ts`, `query/utils.ts` and ~40 unused `Q` methods (traversal,
insertion, show/hide, extra event helpers). Kept: `find`/`parent`/
`closest`/`next`, `append`/`remove`/`replaceWith`, `attr`/`prop`/`val`,
class/css helpers, `on`, `toArray`/`get`. See `frontend/src/query/README.md`.

**Phase 6 — Rust backend.** Flattened `app/updater/updater/` into
`app/updater/` (fixes `module_inception`); shared helpers
`amdgpu_cards()` (AMD card discovery), `rename_key()` (config migration),
`soundboard_setting()` (audio_method reads), `read_colors_db()`
(colors.json load). Removed provable no-op prefix re-strips in
`get_params` (soundboard tests pin the behavior). Zero clippy warnings.

**Phase 7 — dependencies & assets.** Dropped 6 unused Rust deps (test-only
`tower` main entry, `libloading`, `symphonia`, `hound`, `ffmpeg-sidecar`,
`base64`; ~15 packages pruned from `Cargo.lock`). Frontend: removed
definition-only `asNumber()`/`editModalState()`, de-exported 9
module-internal functions. Kept all of `static/` (button images resolve
from user config data, so no file is provably unused) and the tracked
`frontend/demo/` showcase images.

**Phase 8 — gates.** `frontend/knip.json` + pinned `knip` devDependency
(+ `npm run knip`); `.github/workflows/ci.yml` runs fmt/clippy/test and
typecheck/knip/vitest/build. Knip triage removed a dead `raw()` helper,
trimmed both barrels to live re-exports, and de-exported in-file-only
functions/types — knip now reports zero findings.

## Bug fixed during the audit

Folder deletion was broken: `FolderDeleteIcon.svelte` emits a string
`onclick="…deleteFolder('…')"`, `globals.d.ts` declares the global, and
`editor/void.ts` implements it — but `installGlobals()` never assigned
`window.deleteFolder`, so every click threw. One-line fix in
`src/app/wireup.ts` plus a `wireup.test.ts` regression test. The stale
`goFullscreenIFRAME` declaration (no emitter, no implementation) was
removed instead of wired.

## Validation

- `cargo fmt --all -- --check`, `cargo clippy --all-targets
  --all-features -- -D warnings`, `cargo test --all-targets`
  (101 passed, 1 ignored + integration tests).
- `cd frontend && npm run typecheck && npm run knip && npm run test
  && npm run build` (351 vitest tests across 48 files).
- Playwright demo tour (`demo-video.spec.ts`) passes; the 3D specs were
  not re-recorded but their entry (`demo-3d/`) is untouched.
- One pre-existing unhandled error in the vitest run (`Cannot read
  properties of null (reading 'schedule')`, a Svelte `$effect` teardown
  race) predates this refactor and is unchanged.

## Remaining opportunities (deliberately deferred)

- `commands/mod.rs`: the 450-line `handle_command` if-else chain is the
  biggest Rust smell, but splitting it risks `/kill`, `/screensaver`,
  `/copy`/`/paste` quirks — needs regression tests first.
- `on_start/utils.rs` Windows shortcut code (raw pointers,
  `CoTaskMemFree`) and all `cfg(windows)`/`cfg(linux)`-only paths were
  reviewed by reading only; verify on Windows.
- `thiserror`/`anyhow`/`tracing` adoption: no measurable win found over
  the current `log()` + `Result<_, String>` style.
- Playwright specs record demos; they assert almost nothing. Real
  assertion coverage for themes/backgrounds still rests on vitest
  interaction tests.
- `static/img/` likely contains orphaned variants (`TEMP*.png`,
  numbered duplicates), but filenames are user-config-addressable, so
  nothing was deleted. Safe cleanup needs a telemetry/migration pass.
