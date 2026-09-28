# State & Config

## Files

| Path | Role |
| --- | --- |
| `webdeck/config_default.json` | Shipped defaults; seed for first run |
| `.config/config.json` | Live config (created from defaults, migrated, saved) |
| `webdeck/commands.json` | Button catalog (+ plugin merge at load) |
| `webdeck/colors.json` | Color-name DB (optionally sorted on start) |
| `webdeck/version.json` | Version history + updater file directives |
| `webdeck/translations/` (+ `misc/`) | `.lang` files; active dict chosen by `settings.language` |
| `.config/user_uploads/` | Uploaded backgrounds/files (`**uploaded/` URIs) |
| `.config/themes/` | User CSS themes (`*.css`) |
| `.config/plugins/` | rhai plugin scripts |
| `static/` | Served static tree (`/static/*`) |
| `temp/` | Scratch (nircmd, portable staging, legacy args shim) |

`$WEBDECK_CONFIG_DIR` overrides the `.config` dir (test seam so unit tests
never touch the real config).

## Load / save (`src/app/utils/settings/`)

- `get_config.rs`: `ensure_config_exists()` (moves legacy root `config.json`
  into `.config/`, else seeds from defaults), `get_config(check, save)`
  (read + `check_config_update` + optional save), `get_port()` (`--port`
  wins, else `url.port`, else 5000), `save_config()` (pretty JSON, atomic
  temp-file + rename so concurrent readers never see a torn file).
- `save_config.rs`: canonical save with global-variable sync.
- `check_config_update.rs`: migrate obsolete configs to the current schema.
- `merge_dicts.rs`: deep merge used by `POST /save_config`.
- `gridsize.rs`: `update_gridsize(config, h, w)` / `unmatrix` — resizing
  reflows `front.buttons`; depends on folder *insertion order*, which is why
  `serde_json` enables `preserve_order`.
- `create_folders.rs`: materializes the queued `folders_to_create` entries.
- `audio_devices.rs`: input/output device enumeration.

Like Python (where config errors are fatal), IO/JSON failures panic with a
clear message instead of threading `Result` through every caller.

## Config schema (highlights)

- `settings`: `language` (`system` default), `show_popup`, `windows_startup`,
  `windows_start_menu_shortcut`, `auto_updates`, `update_channel`,
  `update_repo`, `dev_mode`, `app_admin`, `optimized_usage_display`,
  `gpu_method`, `show_console`, `open_settings_in_integrated_browser`,
  `data_transfer_method` (`http`), `automatic_firewall_bypass`,
  `sort_colors_on_startup`, `fix_stop_soundboard`, `ear_soundboard`,
  `allowed_networks` ([] + `netmask`), `spotify_api.*`, `soundboard.*`,
  `obs.*`.
- `front`: `background` (list; `//` = disabled, `**uploaded/` = upload),
  `computer_usage_reload_time` (ms), `edit_buttons_color`, `buttons_color`,
  `names_color`, `show_names`, `portrait_rotate`, `themes`, `height`,
  `width`, `dark_theme`, `buttons` (`{folder: [slots…]}`, slot =
  `{image, image_size, message, name, …}`).
- `url`: `port`, `ip` (`local_ip` = auto-detect and write back).

See [reference](reference.md) for the full field table.

## Themes & languages

- `utils/themes/parse_themes.rs`: parses `.config/themes/*.css` + shipped
  themes for `/api/boot parsed_themes`; the editor pushes
  `static/css/style.css` at render/boot time.
- `utils/languages.rs`: `init(dir, misc, lang)`, `get_languages_info()`,
  `lang_dict(active)`, `set_default_language()`; tray language flips on
  config save. `utils/translate.rs` resolves `text:key` references.
