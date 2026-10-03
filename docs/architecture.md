# Architecture

The Rust library exposes feature modules directly from `src/lib.rs`.

| Responsibility | Module |
| --- | --- |
| Generated contracts | `contracts.rs` |
| Schema and semantic validation | `domain.rs` |
| Atomic config transactions and confined assets | `storage.rs` |
| Device grants and hashed tokens | `sessions.rs` |
| Admission, execution and shutdown | `executor.rs` |
| Native effects and integrations | `native.rs`, `native/` |
| HTTP and Socket.IO | `server.rs`, `server/realtime.rs` |
| Tray and QR | `desktop.rs`, `qr.rs` |
| Verified updates and rollback | `update.rs` |

The Svelte app uses `App.svelte`, `Fields.svelte`, `editor.svelte.ts` and `api.ts`. Its global stylesheet is `frontend/src/style.css`; Vite emits `frontend/dist`. Both server and client contracts come from `contracts/v2.schema.json`.

The binaries are `webdeck`, `console`, `update`, `webdeck-qr` and the development packager `package`. Runtime data lives in the selected config directory. Shipped data consists of v2 defaults, translations, version metadata and application icons.
