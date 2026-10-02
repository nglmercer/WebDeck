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
| Typed native actions, OS helpers and audio | `src/native.rs`, `src/native/` |
| HTTP and correlated Socket.IO transport | `src/server.rs`, `src/server/realtime.rs` |
| Native tray and QR viewer | `src/desktop.rs`, `src/qr.rs`, `src/bin/qr.rs` |
| Verified staging, installation and rollback | `src/update.rs`, `src/bin/update.rs` |
| Deck, editor, settings and controller state | `frontend/src/App.svelte`, `editor.svelte.ts`, `api.ts` |

## Execution policy

Admission bounds waiting and running requests together (16 accepted roots). A
single native execution gate deliberately serializes root effects. Nested script
and plugin calls inherit that root's slot and gate, so they cannot bypass
serialization or deadlock while acquiring another worker. Each nested call checks
capabilities, depth and the deadline. Rhai additionally bounds operations, call
levels, expressions, strings, arrays and maps. Plugin capabilities narrow the
caller grant; plugin code and catalog metadata are compiled/loaded at startup.

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
trust. Invalid supplied credentials cannot use that exception. Device approval,
revocation, native selection and Spotify connection approval require a local
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

Current evidence is under `docs/v2/evidence/rewrite/`. Evidence under `v2-only/`
and the historical proposal describes the superseded partial implementation.
