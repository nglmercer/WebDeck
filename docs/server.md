# Server

axum port of the Flask app (`src/app/server/`). Same paths + methods.

## Route table (`mod.rs` → `app_router`)

| Method | Path | Handler | File |
| --- | --- | --- | --- |
| GET | `/` | `home` — serves `frontend/dist/index.html` | routes_boot |
| GET | `/api/boot` | `boot` — full SPA context as JSON | routes_boot |
| POST | `/usage` | `usage` — live system stats | realtime |
| POST | `/send-data` | `send_data_route` — run a command | realtime |
| GET+POST | `/get_config` | `get_config_route` — config (+ folder queue flush) | routes_config |
| POST | `/save_config` | `saveconfig` — merge + migrate + save | routes_config |
| POST | `/COMPLETE_save_config` | `complete_save_config` — full replace, keep grid, resize | routes_config |
| POST | `/save_single_button` | `save_single_button` — one slot by folder/index | routes_config |
| POST | `/save_buttons_only` | `save_buttons_only` — replace `front.buttons` | routes_config |
| POST | `/create_folder` | `create_folder` — queue a button folder | routes_config |
| POST | `/upload_folderpath` | browse-pick a folder path | routes_upload |
| POST | `/upload_filepath` | browse-pick a file path | routes_upload |
| POST | `/upload_file` | multipart upload → `.config/user_uploads/` | routes_upload |
| GET | `/.config/<dir>/<file>` | `get_config_file` — serve uploads/themes/plugins | routes_upload |
| — | `/static/*` | `ServeDir("static")` (Flask static folder) | mod |
| — | `/assets/*` | `ServeDir("frontend/dist/assets")` (SPA bundle) | mod |

`DefaultBodyLimit` is disabled (uploads); tests drive `app_router` via
`oneshot` (`tower::ServiceExt`).

## Boot context (`GET /api/boot`)

`boot_context()` assembles what Jinja used to inline into the page:

- `config` (fresh, migrated + saved), `commands` (catalog + plugins),
  `versions` (`webdeck/version.json`), `langs`, `lang` (active dict),
  `svgs`, `themes` (`.config/themes/*.css` names), `parsed_themes`,
  `random_bg` (random non-`//` entry; `**uploaded/` hits get a rotated copy),
  `usage_example` (`get_usage(example=true)`), `audio_devices`
  (`{input, output}`), `dark_theme` (`""` or `" dark-theme"`), `is_exe`
  (`!cfg!(debug_assertions)`), `portrait_rotate`.
- Side effects: `load_plugins`, wallpaper pick/rotation, and an unconditional
  `static/css/style.css` push onto `front.themes` (fresh config per request,
  so no accumulation).

## Config routes (`routes_config.rs`)

- `saveconfig`: resize grid (`update_gridsize` + store height/width), detect
  soundboard/OBS/language changes, toggle the startup shortcut
  (Windows `.lnk` / Linux `.desktop`, release only), `merge_dicts(old, new)`,
  flush the `folders_to_create` queue (`create_folders`), `check_config_update`,
  `save_config` (twice when `front.background` arrives stringified — parsed
  with the Python quote-normalization first), then `obs::reload_obs()`,
  tray-language switch, and mic stop/restart as needed.
- `complete_save_config`: full replace but keeps old height/width during
  folder creation, then resizes to the new grid.
- `save_single_button`: body `{location_Folder, location_Id, content}`;
  folder is addressed by *index* into `front.buttons` key order (insertion
  order matters — `serde_json` uses `preserve_order`).
- `get_config_route` also flushes pending folders, syncs the `config` global,
  and saves (same as Python).
- `create_folder`: body `{name, parent_folder}`; rejects duplicates already
  queued or already in config; the queued entry is materialized by the next
  save/get_config via `create_folders`.

## Realtime (`realtime.rs`, socketioxide 0.18)

- `POST /usage` → `get_usage(None, &[])` JSON.
- `POST /send-data` → body `{message}` → `spawn_blocking(handle_command)` →
  JSON result. The HTTP path emits no Socket.IO events.
- Socket.IO namespace `/`: `connect` logs; `send` broadcasts `message` to
  others; `message_from_socket` runs `handle_command` off-runtime and emits
  `json_data` with the **original message** (not the result), like Python.

## Middleware (`middleware.rs`)

- `check_local_network` (before): LAN-only guard. Intentional deviation:
  Python tested `remote_ip in ipaddress.ip_address(network)` for
  `allowed_networks`, which raises `TypeError`; Rust implements the intended
  CIDR-contains semantics.
- `after_request` (after): logging, skips `/usage`.
- `internal_error`: always-JSON 500 (`{success:false, message}`); the Flask
  `flask_debug` HTML path is dev-only and not mirrored.

## Bind

Host = `--host` or detected local IP; port = `--port` or `url.port`
(default 5000). The Socket.IO layer wraps the router before
`axum::serve` with `ConnectInfo<SocketAddr>`.
