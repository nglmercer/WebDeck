# WebDeck v2-only migration status

Outcome: **incomplete; draft implementation, not ready to merge or release**.
Recorded 2026-09-30. Starting commit: `d86e5c730393854da06c16ad4791e5249b6842d8`.
Working branch: `codex/v2-only-migration`, based on `v2`. The deprecated v1
branch was not changed. This report supersedes prior compatibility-preserving
requirements. The current implementation does not yet satisfy the requested
one-generation architecture.

## Implemented changes

- Authorization no longer depends on route generation or a legacy policy.
  Remote protected operations require a paired identity. Explicit invalid HTTP
  or socket credentials cannot inherit loopback trust. Loopback Host/authority
  is checked even when a browser omits Origin on a GET. Origin, network,
  capability, expiry, revocation and loopback device-administration checks remain.
  Native file selection additionally requires local interaction.
- `/api/v2/boot` replaces `/api/boot`. Read/input controllers can load the deck
  without Settings. Controller responses omit administrative settings, OBS and
  Spotify configuration, command catalog metadata and audio-device discovery.
  `/api/v2/settings/boot` requires Settings. Plugin initialization runs at
  startup; boot reads catalog snapshots and cannot trigger plugin initialization.
  Editing controls are unavailable to
  controller clients. Config/revision are captured from one service snapshot.
- `/send-data` and the root Socket.IO namespace are unmounted. Both local and
  paired command callers use `/api/v2/commands` or `/v2` `command` events.
  HTTP submission no longer falls back. Socket observations correlate request
  IDs, distinguish acceptance/completion/failure, and report disconnect/timeout
  uncertainty. Volatile submission does not queue offline commands for replay.
  This remains a **string command request**, not the final typed action union.
- Accepted executor work owns admission through both queued and running states,
  even after transport cancellation. The API-generation `strict` switch is gone;
  unknown commands fail. Worker/resource limits and shutdown draining remain.
- Configuration startup/reload/writes require `schema_version: 2`. Missing/old
  versions fail without conversion, backup creation/restoration, replacement or
  deletion. Existing root `config.json` is left untouched and blocks first-run
  default creation. Normal key renames, hyphen normalization, Python list parsing,
  string-boolean coercion, GPU alias conversion and defaults-merging were removed
  from the configuration-normalization module. Useful theme discovery remains.
- Every ConfigService mutation requires a revision. Remaining HTTP mutation
  handlers require a valid revision header; v2 replacement requires its body
  revision. Missing/stale client revisions fail. Reads do not flush writes or
  apply settings. Folder creation publishes an explicit transaction immediately,
  preserving existing editor drafts. Configuration resizing and settings
  publication moved from transport to `application/layout.rs`.
- Rust, frontend and UI version metadata use `2.0.0-alpha.1`. Development
  portable labels include that version. Update candidates must be maintainer
  `nglmercer/WebDeck` v2 prereleases with matching platform/version artifact URL
  and digest. Semantic version ordering handles numbered prereleases. Automatic
  polling is disabled until the distribution path is verified. Verified staging,
  constrained extraction, installation journaling and rollback remain.

The [current action inventory](ACTION_INVENTORY.md) lists registry actions,
capabilities, frontend-only flows and their remaining migration ownership.

## Migration map and unfinished work

| Old surface | Current replacement / removal | Consumers and evidence | Remaining blocker |
| --- | --- | --- | --- |
| `/send-data` | Unmounted; `/api/v2/commands` | HTTP frontend, console, demo, performance harness; authorized-caller 404 test | Request still contains `message`; native parsing remains |
| `/api/boot` | Unmounted; `/api/v2/boot`, `/api/v2/settings/boot` | SPA, acceptance, portable, performance; least-privilege HTTP/browser checks | Boot/config models still loose; privileged and safe boot share a builder |
| Root namespace, `message_from_socket`, `send`, `json_data` | Root namespace removed; `/v2` `command` / `command_result` | SPA and real Socket.IO acceptance; retirement, lifecycle, expiry/revocation, disconnected no-replay | Command payload remains string-based; frontend runtime schema validation unfinished |
| `/save_config`, `/COMPLETE_save_config`, `/save_single_button`, `/save_buttons_only` | **Still mounted**, now revision-required | Settings/editor wrappers, demos and browser tests | Canonical typed mutations and stable button identifiers needed |
| `/get_config` | **Still mounted**, now read-only snapshot | Editor loads and demos | Replace clients with canonical v2 config/deck reads |
| `/create_folder`, queued folder writes | Queue removed; immediate revision-required transaction; route **still mounted** | Add-button flow and editor draft merge | Canonical v2 folder mutation and explicit ordering needed |
| `/usage` | **Still mounted** | Usage tiles and frontend polling | Typed v2 usage endpoint and request model needed |
| `/upload_file`, `/upload_filepath`, `/upload_folderpath` | **Still mounted**; local-only native pickers | Upload widgets and existing malformed multipart tests | Typed upload/selection operations and canonical assets needed |
| `.config/`, `**uploaded/`, external path inference | **Still interpreted** | Assets, soundboard, scripts, editor and background UI | Separate managed asset references from privileged external paths |
| `parse_legacy`, prefix aliases, `<\|§\|>` | **Still present** | Executor, native/integration adapters and legacy console syntax | Generate discriminated action contracts; pass typed adapter arguments |
| `app/buttons` facade and direct `handle_command` | **Still present** | Scripts/plugins/internal callers | Explicit nested execution with authorization/resource coordination and regression tests |
| Old config shape under schema 2 | Version gating added, conversion removed | Defaults and persistence tests | Complete generated config model; typed buttons/actions, IDs/order and strict complete-shape validation |
| Plugin `_dict_doc`, `.py` discovery, delimiter host API | **Still present** | Loader, examples and plugin adapter | One explicit v2 plugin identity/action/argument/result/capability contract |
| Python-named Rhai modules, `type:uploaded_file`, `type:file_path` | **Still present** | Script metadata, loader and Rhai host | Typed script sources and nested invocation design |
| Legacy security selector / tokenless remote exception | Removed from authorization and settings UI | HTTP, socket and controller tests | Full policy audit after all new mutation surfaces exist |
| Old stable updater source / v1 candidate selection | Replaced, automatic polling disabled | Release selection tests, version metadata and packaging | Real signed Windows/Linux v2 distribution validation |
| Legacy golden contract/tests | **Still present** | `contracts/legacy-commands.json`, parser/native tests | Replace positive compatibility assertions after typed action migration |

Internal convenience saves still capture their revision at save time; consolidating
those writers with the original read snapshot is part of the remaining shared
configuration/global-state work. The complete generated contract constraints and
runtime validation must also be made consistent.

The production guard scans sources and live/demo tooling, not historical
Markdown or explicit rejection tests. `node tools/validation/v2-only.mjs` returns
**nonzero** while remaining compatibility surfaces are present. CI runs that
completion gate; passing ordinary tests does not make this migration complete.
The machine-readable violations are in `evidence/v2-only/migration-guard.json`.

## Validation evidence

The baseline passed the existing Rust suite and 379 frontend tests. The initial
plain-shell Cargo invocation failed because Cargo was not on PATH; subsequent
commands used `/workspace/.webdeck-tools/env.sh` (installed Rust and system-library
selectors). No test/lint settings were relaxed. Intermediate compile/type/test
failures were repaired before the final run.

See `evidence/v2-only/checks.json` and the corresponding independent logs for
commands, exit codes and timings. Final validation and artifact results are
recorded in [MIGRATION_REPORT.md](MIGRATION_REPORT.md). Earlier evidence files
outside `evidence/v2-only/` describe the baseline compatibility implementation
and are not evidence for these changes.

Persistence tests cover unsupported-version preservation, existing backup
preservation, revision conflicts, independent advisory-lock owners, atomic-write
failure and retention of the last valid snapshot after invalid external edits.
Advisory locks only coordinate application writers; editors that ignore them
can race publication. Do not edit the file concurrently with an application save.
Executor tests cover both resource-lock and worker waiting after caller abort.
HTTP/native command coverage uses fake effects. Browser acceptance uses a real
HTTP/Socket.IO server with fake native effects and isolated data.

## Platform and release gates

Windows checks were not run in this Linux workspace. Native key/mouse input,
window/power effects, soundboard/microphone/device-loss, live OBS/Spotify,
representative real plugins/scripts, signed Windows packaging and Windows
installation upgrade/rollback require separate platform/credential/hardware
validation. Automatic updates stay disabled. No release or artifact was
published. Installation rollback safety is independent of v1 config conversion
and remains available.

These external gates are additional to the **unfinished code migration** above;
this branch is not code-complete awaiting only hardware validation. Use an
isolated data directory for evaluation. Never relabel an old configuration as
schema 2 to bypass rejection; preserve old data and use the deprecated branch.
