# WebDeck v2 implementation status

Implementation branch: `v2`, based on `343baf22bccbcb26c6114a5795d1449c2d83a777`.
Recorded on 2026-09-30. This is an integration implementation, awaiting release
validation. Stable release metadata, version `1.8.7` and update channels have
not changed. `master` remains the v1 maintenance line.

The [proposal](README.md) and [original plan](IMPLEMENTATION_PLAN.md) retain the
design rationale. This document describes the code actually implemented and
distinguishes automated evidence from release approval.

## Architecture and ownership

- `src/domain`: typed commands, capabilities, resources, validated configuration,
  stable errors and generated transport requests. No native effects.
- `src/application`: one shared configuration owner, optimistic revisions,
  bounded execution, pairing sessions and shutdown coordination.
- `src/adapters`: durable filesystem writes, confined assets, verified updater,
  native platform effects and OBS/Spotify/script integrations.
- `src/app/server`: HTTP and Socket.IO compatibility adapters plus versioned
  routes. `src/app/buttons` is a compatibility facade; native implementations
  were removed from that tree after moving to their adapter owners.
- `frontend/src/features`: deck navigation, editor persistence, settings
  submission, pairing and modal state. Existing views consume typed callbacks.
  There are no application functions registered on `window` or executable
  command attributes. DOM compatibility helpers remain for the old markup.

Native effects run through the shared command executor (16 admitted requests,
4 workers). Commands sharing a resource serialize. Cancellation does not release
capacity while a blocking effect continues. Shutdown closes admission, drains
accepted work, stops owned shell process groups, microphone and sound-player
workers. OBS/Spotify calls and synchronous shell work have 30-second limits;
scripts have operation, depth and collection limits. Side effects are never
automatically replayed or retried. GUI windows and external applications opened
by the OS remain subject to native platform behavior; verify their lifecycle
in the desktop smoke checklist.

## Compatibility and intentional changes

| Surface | Implemented behavior | Evidence / release limit |
| --- | --- | --- |
| Legacy command strings and aliases | Typed compatibility parser preserves ordering, original text and placeholder/tail behavior; unknown legacy messages retain success behavior | `tests/v2_commands.rs`, legacy command unit tests |
| HTTP command execution | Legacy final result preserved; v2 uses stable error codes and request IDs | `tests/v2_http.rs`, contracts test |
| Socket.IO | Legacy `json_data` echo retained; v2 `command_result` sends accepted then completed/failed; no offline replay | Chromium socket test and executor tests |
| Configuration | Schema 1 migrates idempotently to 2; extra data and folder ordering preserved; immutable v1 backup; revision conflicts preserve editor drafts | 7 config tests and real-browser conflict test |
| Editor/settings | Create, edit, delete, swap, navigation, appearance, themes, backgrounds and grid dimensions persist | Frontend suite and 3 Chromium acceptance tests |
| Modal lifecycle | State owns visibility; focus trap, focus restoration, listener/observer cleanup; refresh replaces owned theme stylesheet links | Unit tests and browser repeated open/close |
| Uploads/assets | Capped multipart body, field validation before publication, rooted paths and symlink rejection | HTTP malformed/duplicate/traversal fixtures; native picker remains desktop-dependent |
| Rhai/scripts/plugins | Existing compatibility commands retained behind capability checks and execution limits | Parser/adapter tests; real plugin behavior needs desktop smoke |
| OBS/Spotify/audio/input/window/power | Implementation extracted into native/integration adapters; no claim of live integration parity from mocks | Hardware and credential-dependent checks required below |
| Portable/update | Fresh frontend/native builds, SHA-256 manifest, constrained verified staging, journaled upgrade and rollback, private user config preserved | Update tests, rollback CLI and actual extracted Linux artifact smoke |

External manual config edits reload only when valid; an invalid edit does not
replace the last valid in-memory snapshot. All application writers use an OS
advisory lock and atomic publication. External editors do not honor that lock;
there remains a small race between an external edit and atomic replacement.
Do not edit the file concurrently with an application save.

The compatibility facade and legacy JSON/DOM models deliberately remain until
native parity evidence permits removal. A directory move alone is not evidence
that hardware behavior has been validated.

## Pairing and security

`settings.v2_security` defaults to `legacy` during migration. This retains the
existing LAN policy for legacy clients. Use Settings → Experimental → Manage
devices on the local host to approve a device and switch to `paired` mode.
Pairing tokens are shown once and stored as hashes in a private device file;
they are sent in headers/socket authentication, never URLs. Grants have
capabilities and expiration; revocation and expiry are rechecked for every
socket command. Local approval/revocation endpoints cannot be used by remote
peers. Browser origins must match the HTTP host; forwarding headers do not
make a remote peer local. Loopback browser requests must use localhost or a known
server IP, preventing DNS rebinding from inheriting local administrative trust.
Custom DNS hostnames on the hosting computer must use localhost/the server IP
instead. IPv4 and IPv6 CIDRs are validated explicitly.

V2 remote routes always require a token. In paired mode, legacy remote clients
also require it. Existing clients without pairing support consequently need
updating before enabling paired mode. Read does not imply command execution;
power, scripting and settings use separate grants. Settings is a trusted grant:
it can read integration configuration containing credentials. Remote Settings
grants cannot change pairing, network/admin/firewall or update-channel policy.
There is no replay after reconnect and no automatic retry of failed effects.
Debug-only `WEBDECK_FAKE_EFFECTS=1` enables headless acceptance; release binaries
ignore it. A focused maintainer security review is still a release gate.

## Migration and recovery

Back up the entire existing data directory, including uploads, themes and
plugins, before changing branches. Start v2 with the same directory, or set
`WEBDECK_CONFIG_DIR` to an isolated copy for evaluation. The first valid load
creates `config.v1.backup.json` once and migrates schema metadata/defaults.
Configuration and pairing writes use private permissions on Unix; validate
Windows ACLs in a real installation. Do not copy tokens into issue reports.

To return to v1, close v2, preserve the current data directory separately, and
restore the v1 backup to `config.json` alongside the backed-up assets. New v2
edits are not present in that original backup. The configuration service also
exposes an explicit backup-restoration operation for local tooling.

Updater archives cannot replace user configuration. Downloads require GitHub
HTTPS and the release asset SHA-256 supplied by trusted release metadata.
A digest verifies integrity; it is not an independent publisher signature.
Windows release packaging requires an independently obtained NirCmd digest
and successful binary signing. Development archives are explicitly named
`dev-portable`. No artifact has been published from this branch.

Close the application before restoring an updater backup:

```sh
/path/to/update --rollback /path/to/backup --destination /path/to/installation
```

Keep administrator ownership of backup and installation paths. The updater
records originals before changing files and retains successful backups.
Windows locked files fail safely. The automatic launcher stops the app before
its download preflight; download failure leaves the installation unchanged but
requires restarting the app manually. Rehearse this on Windows before release.

## Reproducible checks

Install the native GTK/WebKit/ALSA/XCB development libraries listed in CI and
use current stable Rust and Node 22+:

```sh
cargo fmt --all -- --check
node tools/contracts/generate.mjs --check
cargo test --locked --all-targets
cargo clippy --locked --all-targets --all-features -- -D warnings
npm ci --prefix frontend
npm ci --prefix tools/component-check
npm --prefix frontend run check:components
npm --prefix frontend run typecheck
npm --prefix frontend run knip
npm --prefix frontend test
npm --prefix frontend run build
cargo build --locked --bin webdeck
(cd frontend && npx playwright install chromium && npm run test:acceptance)
cargo run --locked --bin package
node tools/validation/portable.mjs
WEBDECK_PORTABLE_ARTIFACT="$PWD/dist/WebDeck-linux-x86_64-portable.zip" \
  cargo test --locked --test v2_update portable_archive -- --ignored
```

The cloud run uses installed `/usr/bin/chromium`; set `WEBDECK_CHROMIUM` for
another executable. Portable smoke currently requires Python 3 and Chromium
and was executed on Linux x86_64. Windows CI is configured but has not been
executed in this Linux workspace. No supported Windows release is inferred.
The final Linux run passed 137 Rust tests (2 opt-in checks ignored) plus the built-artifact upgrade/rollback check, 379 frontend
tests in 53 files, all 3 Chromium acceptance tests, formatting, generated
contracts, Clippy, TypeScript, Knip and Svelte diagnostics (0 errors/warnings).

Evidence is in [evidence/](evidence/): original baseline, final checks,
[portable artifact report](evidence/portable.json), and
[performance comparison](evidence/performance.json).

## Performance and remaining release gates

The recorded sequential debug-profile comparisons use frozen baseline binaries
and frontend, the same isolated config, 30 measured boot/command requests after
warmup, 5 browser loads, RSS and idle CPU. The latest run measured boot p95 at
56.7 ms baseline / 33.6 ms v2, usable UI p95 at 1752.2 / 1714.3 ms, and command
p95 at 10.41 / 3.81 ms. Compressed JS grew from 76,635 to 80,415 bytes; RSS was
84,280 / 83,932 KiB and both measured zero idle CPU ticks in one second.

The [initial run](evidence/performance-initial.json) instead measured boot p95
at 31.9 / 48.7 ms and UI p95 at 1594.5 / 1907.5 ms. These shared cloud measurements
show substantial variability and do not establish a speedup or an agreed
performance budget. Review boot/UI under controlled production workloads and
establish budgets before release; bundle growth is about 4.9%.

Before stable cutover, record Windows CI and extracted-artifact results; test
native key/mouse input, window operations, tray/popup/QR shutdown, device loss,
loopback microphone and soundboard on Windows and Linux; exercise live OBS and
Spotify with disposable credentials; validate representative Rhai/plugins;
rehearse an upgrade and rollback of a real Windows installation; review token,
origin, capability and archive handling; approve performance budgets and release
notes. A maintainer must approve stable cutover. This branch does not satisfy
those external/native release gates merely because automated tests pass.
