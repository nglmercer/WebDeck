# Frontend — Shell

> V2 implementation: see [architecture, compatibility and validation](v2/STATUS.md). The historical descriptions below document the v1 compatibility layer.

Svelte + TypeScript SPA (`frontend/`). Zero-dependency custom framework plus
a jQuery-like `query/` layer — a 1:1 port of the old Jinja/JS behavior.

## Boot (`src/main.ts` → `views/app.ts`)

1. Mount `#app`; show `LoadingScreen` (same visual sequence as Jinja).
2. `GET /api/boot` → `BootContext` (`framework/types.ts`).
3. `initI18n(ctx.lang)`; unmount loading; `renderApp(mountEl, ctx)`.
4. On failure: inline error in `#app` and rethrow.

`GET /` serves `frontend/dist/index.html`; without `npm run build` it returns
a JSON error telling you to build. `/assets/*` serves the bundle.

## Shell views

- `App.svelte` / `app.ts` — root render + view switching.
- `Grid.svelte` / `grid.ts` — button grid (`front.height × front.width`),
  folder navigation (`/folder <name>`), zoom/auto-zoom, portrait rotation,
  random background (`random_bg`), dark theme (`dark_theme` class).
- `Shell.svelte` / `shell.ts` — chrome around the grid (top bar, config
  entry, connection/reconnect screen when the server is unreachable).
- `FoldersBar.svelte` — folder tabs above the grid.
- `LoadingScreen.svelte` — boot splash (SVG preload avoids display flashes).
- `Config.svelte` / `config.ts` — settings editor (sections, backgrounds,
  themes, devices, danger zone); save flows POST to `/save_config`,
  `/COMPLETE_save_config`, `/save_buttons_only`, `/save_single_button`.
- `BackgroundsPanel.svelte` — add-background composer: Color/File tabs
  (`StudioTabs`); owns the background list state (toggle/delete/add/upload)
  bound to the config-form handler input.
- `ThemesPanel.svelte` — owns the theme list state (enable/disable/reorder)
  bound to the config-form handler input.
- `ThemesPanel.svelte` — theme manager (enable/order `.config/themes/*.css`).
- `Preview.svelte` / `preview.ts`, `studio-preview.ts` — button/studio previews.

## State & wireup

- `api/` — one HTTP layer: `client.ts` transport (`getJson`/`postJson`/
  text/form, typed `HttpError`, timeout/abort) plus `config.ts`
  (`/api/boot`, `/get_config`, `/save_config`), `buttons.ts`
  (`/save_buttons_only`, `/save_single_button`, `/create_folder`,
  `/send-data`), `uploads.ts`, `usage.ts`.
- `framework/i18n.ts` — `initI18n(langDict)` + `t(key)` lookups.
- `framework/html.ts` — typed HTML builders for non-Svelte-rendered parts.
- `query/` — `q`, `byId` DOM facade: `attributes`, `classes-css`,
  `core`, `events`, `factory`, `manipulate`, `traverse` (each with
  `*.test.ts`). Unused jQuery-isms (effects, `data`, static utils,
  `off`/`one`/`trigger`, extra traversals/insertions) were removed.
- Button press → `POST /send-data {message}` (HTTP) or Socket.IO
  `message_from_socket`; `data_transfer_method` selects the path.

## Usage loop

`Grid` polls `POST /usage` every `front.computer_usage_reload_time` ms
(default 3000) and patches usage tiles in place; `/api/boot usage_example`
provides the first-paint snapshot. `settings.optimized_usage_display`
enables extra update skipping. Each applied poll emits `usage:updated`.

## App events

Lifecycle moments are exposed as `window` CustomEvents (`webdeck:<name>`,
see `src/app/events.ts`): `boot:ready`, `app:refreshed`, `usage:updated`,
`editor:changed`, `save:completed`, `server:disconnected`,
`server:reconnected`. In-app modules subscribe via `onAppEvent`; external
consumers (Playwright, user scripts) listen on `window` directly.

## Framework + query tests

`frontend/src/**/*.test.ts` run under vitest (`npm test`, 298 tests):
per-view tests (`app`, `shell`, `grid`, `Config`, `editmodal`, `ArgsBlock`,
`argschema`, `args`, `argvalues`, `getcommand`, `labels`, `svg`,
`LoadingScreen`), per-component tests (fields, icons, preview,
`CollapseSection`, `EditorStyleBlock`, `search-dropdown`), and per-module
`query/` tests. `npm run typecheck` (svelte-check) and `npm run build`
(vite) gate releases.
