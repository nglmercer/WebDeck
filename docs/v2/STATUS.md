# Rebuilt v2 runtime

The application runtime and UI were replaced after the earlier partial migration.
The canonical schema is `contracts/v2.schema.json`; it generates Rust/TypeScript
contracts and the action catalog. The v2 completion guard checks production code
and rejects the discarded application trees and protocols.

| Responsibility | Owner |
| --- | --- |
| Contracts, capabilities and validated IDs | `src/contracts.rs`, `src/domain.rs` |
| Revision-aware configuration and asset confinement | `src/storage.rs` |
| Pairing, expiry, revocation and hashed credentials | `src/sessions.rs` |
| Accepted work, bounded admission and shutdown drain | `src/executor.rs` |
| Embedded JavaScript command behavior | `runtime/src/`, `src/runtime/` |
| Authorized OS, audio and transport primitives | `src/capabilities/` |
| HTTP and correlated Socket.IO transport | `src/server.rs`, `src/server/realtime.rs` |
| Native tray and QR viewer | `src/desktop.rs`, `src/qr.rs`, `src/bin/qr.rs` |
| Verified staging, installation and rollback | `src/update.rs`, `src/bin/update.rs` |
| Deck, editor, settings and controller state | `frontend/src/App.svelte`, `features/`, `lib/` |

## Execution policy

Admission bounds waiting and running requests together (16 accepted roots).
A single VM owner serializes ordinary command execution and synchronous native
calls. Workflow parallel nodes overlap guest promises and timers, while host effects
remain serialized. Metadata, authorization and direct usage queries have separate
bounded workers. Keyboard chords retain their input lock. Nested script and plugin
calls inherit the accepted root without reacquiring admission. Rust rechecks the
original grant, depth and deadline at every host boundary. The embedded VM also
bounds fuel, loops and execution time. Fresh script globals and separate persistent
plugin interpreters prevent state sharing. Local administrators can inspect,
enable/disable external actions and reload verified plugin packages.

Acceptance transfers ownership to a server task before notifying the observer.
Dropping an HTTP/socket observer does not cancel accepted work. Shutdown closes
admission, drains accepted roots and stops application-owned children and sounds.
The frontend uses volatile socket sends and clears pending observers on disconnect;
it never automatically replays a command whose outcome is uncertain.

## Data policy

Only canonical v2 documents are read or written. The default file is created only
when no user configuration exists. There is no v1 parser, conversion, alias,
placeholder or `.config/` URL interpretation. Atomic replacement, process writer
locks, fingerprints and explicit revisions protect configuration mutations.
Invalid external edits leave the last valid snapshot intact and block writes.
Settings restore canonical backups into the editor draft, rather than bypassing
revision checks. User data, uploaded files, themes, plugins and backups are
preserved. Existing incompatible files require an isolated new data directory or
manual recovery outside this runtime.

Protected remote requests require a device identity. The local exception uses the
actual loopback peer, approved literal Host/Origin values and no forwarded-header
trust. Supplied credentials always retain device identity, including from loopback; invalid credentials cannot use that exception. Configuration access and editing require the local administrator. Device approval,
revocation, native selection, integration checks and Spotify connection approval require a local
administrator. Controller boot contains action references and required capabilities,
not action code, HTTP headers or integration settings. Both transports resolve
references against current configuration and authorize every invocation.

## Validation limits

The recorded checks cover deterministic Rust tests, Svelte/TypeScript, Chromium
against a real server with fake effects, Linux development packaging, extraction
and real updater rollback. Cross-target Windows compilation checks native APIs;
it does not establish Windows tray, input, audio or packaging behavior.

Live desktop sessions, audio devices and virtual microphones, power actions,
firewall rules, OBS and Spotify accounts were not exercised in this environment.
Their failures are returned explicitly instead of reporting success. The Linux
artifact requires compatible native system libraries. Signed release artifacts
and automatic-update distribution have not been validated or published.

Current rewrite evidence is under `docs/v2/evidence/rewrite/`. Superseded partial-migration reports have been removed. The subsequent asset/documentation cleanup is recorded in `evidence/rewrite/cleanup.json` and `cleanup-portable.json`.

The subsequent code/design implementation is tracked separately in
[improvement progress](../improvement-progress.md) and its
[completion audit](../improvement-audit.md). Current screenshots, measured browser
workloads and development portable checks live under `evidence/improvements/`.
Those records identify their verification scope; a newer UI build does not make
an older portable checkpoint current. Live platform/account verification remains
open for that plan.
