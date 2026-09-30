# Architecture

> V2 implementation: see [architecture, compatibility and validation](v2/STATUS.md). The historical descriptions below document the v1 compatibility layer.

## Binaries + shared lib

`Cargo.toml` declares one lib and five binaries; all binaries share the same
implementation through `webdeck::app` (`src/lib.rs` only re-exports `app/`):

| Binary | Entry | Purpose |
| --- | --- | --- |
| `webdeck` (default run) | `src/main.rs` | Full app: config, server, tray, popup |
| `console` | `src/bin/console.rs` | Dev console helper (not shipped) |
| `update` | `src/bin/update.rs` | Standalone updater step (shipped) |
| `package` | `src/bin/package.rs` | Portable-zip packager (dev tool) |
| `webdeck-qr` | `src/bin/qr.rs` | QR window (minifb) showing the server URL |

Module layout mirrors the Python `app/` tree 1:1: `app/<path>/<module>.py`
→ `src/app/<path>/<module>.rs`, each `__init__.py` → parent `mod.rs`.

## Backend (`src/app/`)

- `buttons/` — command execution. `commands/mod.rs` owns `handle_command()`;
  siblings own one domain each: `audio`, `color_picker`, `exec`, `obs`,
  `soundboard`, `spotify`, `system`, `usage`, `window`.
- `server/` — axum app: `mod.rs` (router + `run_server`), `routes_boot`,
  `routes_config`, `routes_upload`, `realtime` (Socket.IO + `/usage` +
  `/send-data`), `middleware`, `assets`.
- `on_start/` — first-run/startup tasks returning `(config, commands, local_ip)`.
- `tray/` — tray icon, menu, windows (config/QR/port dialogs).
- `updater/` — `check` (startup check) + `updater/` (download/apply/files/versions).
- `utils/` — `args` (clap CLI), `settings` (config load/save/migrate/gridsize),
  `languages`, `themes`, `plugins` (rhai), `logger`, `working_dir`,
  `get_local_ip`, `firewall`, `qr`, `welcome_popup`, `show_error`, `exit`,
  `restart`, `merge_dicts`, `is_opened`, `kill_nircmd`, `translate`, `debug`.

Config is re-read from disk per handler (like Python); `AppState` only carries
the `folders_to_create` queue and `local_ip`.

## Frontend (`frontend/`)

Svelte + TypeScript SPA, zero-dependency custom framework:

- `src/main.ts` — boot: loading screen → `GET /api/boot` → `renderApp`.
- `src/views/` — `App`, `Grid`, `Shell`, `Config`, `EditModal`, `AddModal`
  (under `addbutton/`), `BackgroundsPanel`, `ThemesPanel`, `FoldersBar`,
  arg schema/values/builders (`argschema.ts`, `args.ts`, `argvalues.ts`).
- `src/framework/` — `api` (fetch helpers), `html`, `i18n`, `types`
  (incl. `BootContext`).
- `src/query/` — jQuery-like DOM helper (`q`, `byId`) with `ajax`,
  `attributes`, `classes-css`, `core`, `data`, `effects`, `events`,
  `factory`, `manipulate`, `traverse`, `utils`.
- `src/components/` — field widgets (`TextField`, `NumberField`, `ColorField`,
  `FileField`, `SelectField`, `SwitchField`, `KeyFieldView`, …) + `studio/`.

Build output goes to `frontend/dist/`; the backend serves `GET /` from
`frontend/dist/index.html` and `/assets/*` from `frontend/dist/assets/`.
If `dist/` is missing, `GET /` returns a JSON error telling you to build.

## Runtime data

- `webdeck/` — shipped defaults: `config_default.json`, `commands.json`
  (button catalog), `colors.json`, `translations/`, `version.json`.
- `.config/` — live state: `config.json`, `user_uploads/`, `themes/`, `plugins/`.
- `static/` — served static files (`/static/*`); `temp/` — scratch
  (args shim cleanup, nircmd staging, portable staging).
