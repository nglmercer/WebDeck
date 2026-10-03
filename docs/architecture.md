# Architecture

The Rust library exposes feature modules directly from `src/lib.rs`.

| Responsibility | Module |
| --- | --- |
| Generated contracts | `contracts.rs` |
| Schema and semantic validation | `domain.rs` |
| Atomic config transactions and confined assets | `storage.rs` |
| Device grants and hashed tokens | `sessions.rs` |
| Admission, execution and shutdown | `executor.rs` |
| Native effects and integrations | `native.rs` dispatch; `native/` input, processes, metrics, network, audio, system, capture, scripts and integrations |
| HTTP and Socket.IO | `server.rs` composition; `server/` auth, commands, configuration, devices, assets, integrations, metadata and realtime |
| Tray and QR | `desktop.rs`, `qr.rs` |
| Verified updates and rollback | `update.rs` |

Async routes share bounded blocking admission without serializing unrelated native
effects. Authorization and network policy have four slots, disk operations have four,
and hardware queries have two. Each worker retains its permit until completion even
if its observer disconnects. HTTP and realtime use the same authorization helper;
supplied credentials are checked against current grants on every invocation and never
fall back to local administration. Saturation fails before effect admission.

Native execution carries capability checks and a shared deadline through nested
commands. Subprocess helpers clamp their timeout to the remaining budget. Process
and audio owners define shutdown and cleanup; prepared audio stays paused until all
outputs are ready and the execution context still permits playback. Synchronous OS
calls cannot always be interrupted, so observer timeout does not prove cancellation
and clients must not automatically replay an uncertain command.

The Svelte app composes `DeckView`, `EditorToolbar`, `ButtonEditorDialog`, and
appearance/integration/device settings components. `Editor` owns draft mutations,
revision and save generations. `AssetCache` owns object URLs and bounds loading;
`UsageMonitor` owns metric polling. `ActionFields` adds searchable action selection
above the recursive `Fields` fallback. `schema.ts` owns schema interpretation;
`api/http.ts` and `api/realtime.ts` own their transport lifecycles, with `lib/api/client.ts`
providing endpoint functions. Generated contracts live in `lib/contracts.ts`.

`Modal` and the shared `modal` action use native dialogs with explicit Tab cycling
and focus restoration. Styling uses `styles/tokens.css` and `styles/base.css`; Vite emits
`frontend/dist`. Both server and client contracts come from `contracts/v2.schema.json`.
`lib/messages.ts`, `lib/translations.ts`, and `lib/i18n.ts` separate stable UI copy,
fallback/interpolation, and per-application reactive translation context.

`Session`, `Navigation`, `DeckInteractions`, `Execution`, and `ButtonDraft` own
loading, hashes, gestures, outcomes, and staged edits respectively. `App.svelte`
composes these owners. Remaining plan work is tracked in the
[implementation progress](improvement-progress.md).


The binaries are `webdeck`, `console`, `update`, `webdeck-qr` and the development packager `package`. Runtime data lives in the selected config directory. Shipped data consists of v2 defaults, translations, version metadata and application icons.
