# Reference

## HTTP API

Base: `http://<host>:<port>` (host = `--host` or local IP, port = `--port`
or `url.port`, default 5000). Errors are JSON
`{success:false, message}` with HTTP 500.

| Method | Path | Request | Response |
| --- | --- | --- | --- |
| GET | `/` | — | `frontend/dist/index.html` (or build-hint JSON 500) |
| GET | `/api/boot` | — | Boot context (config, commands, versions, langs, lang, svgs, themes, parsed_themes, random_bg, usage_example, audio_devices, dark_theme, is_exe, portrait_rotate) |
| POST | `/usage` | — | `get_usage()` snapshot JSON |
| POST | `/send-data` | `{message}` | Command result (usually `{success:true}`) |
| GET+POST | `/get_config` | — | Full config JSON (flushes folder queue, saves) |
| POST | `/save_config` | full config | `{success:true}` (merge + migrate + save) |
| POST | `/COMPLETE_save_config` | full config | `{success:true}` (replace, keep grid, resize) |
| POST | `/save_single_button` | `{location_Folder, location_Id, content}` | `{success:true}` or 500 (bad folder index) |
| POST | `/save_buttons_only` | `{front:{buttons}}` | `{success:true}` |
| POST | `/create_folder` | `{name, parent_folder}` | `{success:true}` or `{success:false, message:"Folder already exists"}` |
| POST | `/upload_folderpath` | picker args | `{…path}` |
| POST | `/upload_filepath` | picker args | `{…path}` |
| POST | `/upload_file` | multipart file | `{…uploaded name}` → `.config/user_uploads/` |
| GET | `/.config/<dir>/<file>` | — | Uploaded/theme/plugin file |
| GET | `/static/*` | — | Static tree |
| GET | `/assets/*` | — | `frontend/dist/assets/` bundle |

## Socket.IO (`/` namespace, socketioxide)

- `connect` — server logs `server connected at <addr>:<port>`.
- `send` (client → server, any JSON) — broadcast as `message` to others.
- `message_from_socket` (client → server, string) — runs `handle_command`,
  emits `json_data` with the **original message** on success.

## CLI

`webdeck [OPTIONS] [exit]` — exit code 2 on parse error, 0 for `--help`:

| Flag | Meaning |
| --- | --- |
| `-v, --version` | Print `WebDeck v<version> (<sha?>)` and exit |
| `-p, --port <u16>` | Override `url.port` |
| `-H, --host <addr>` | Override bind host (default: local IP) |
| `-t, --timeout <secs>` | Self-terminate after N seconds |
| `--no-admin` (`--no-sudo`) | Skip UAC elevation |
| `--no-tray` | Console mode (Ctrl+C to stop) |
| `--no-debug` | Disable debug logging |
| `--force-start` | Bypass single-instance guard |
| `--log-file <path>` | Log to file |
| `--force-update` (`--update`) | Force update check |
| `--no-auto-update` (`--no-update`) | Disable update check |
| `--fake-error` (dev) | Exercise error dialog path |
| `--test-ffmpeg` (dev) | Print ffmpeg path and exit |
| `exit\|stop\|close\|quit\|kill\|terminate\|shutdown` | Stop all instances |

## Config schema (`.config/config.json`)

```jsonc
{
  "settings": {
    "language": "system", "show_popup": true,
    "windows_startup": false, "windows_start_menu_shortcut": false,
    "auto_updates": true, "update_channel": "stable",
    "update_repo": "Lenochxd/WebDeck", "dev_mode": false,
    "app_admin": true, "optimized_usage_display": false,
    "gpu_method": "nvidia (NVML)", "show_console": false,
    "open_settings_in_integrated_browser": false,
    "data_transfer_method": "http",
    "automatic_firewall_bypass": false,
    "sort_colors_on_startup": false, "fix_stop_soundboard": false,
    "ear_soundboard": true, "allowed_networks": [], "netmask": "16",
    "spotify_api": {"client_id": "", "client_secret": "", "username": ""},
    "soundboard": {"enabled": false, "audio_method": "vlc",
                   "mic_input_device": "", "vbcable": "cable input"},
    "obs": {"host": "localhost", "port": 4455, "password": ""}
  },
  "front": {
    "background": ["#141414"], "computer_usage_reload_time": "3000",
    "edit_buttons_color": false, "buttons_color": "",
    "names_color": "#b3b3b3", "show_names": true,
    "portrait_rotate": "90", "themes": [],
    "height": "4", "width": "8", "dark_theme": true,
    "buttons": {"index": [
      {"image": "folder.png", "image_size": "70%",
       "message": "/folder folder1", "name": "Folder 1"}
    ]}
  },
  "url": {"port": 5000, "ip": "local_ip"}
}
```

Notes: `background` entries starting with `//` are disabled;
`**uploaded/` prefixes resolve into `.config/user_uploads/`.
`front.buttons` key order is significant (positional folder addressing).

## Command message formats

- General: `/<command>[ <args>]`; `<|§|>` → space before dispatch.
- `/usage '<title>' …` tile templates embed `usage_dict[…]` lookups
  (see `commands.json` Display category).
- `/volume …`, `/appvolume +|-|set …`, `/setmicrophone <name>`,
  `/setoutputdevice <name>`, `/key <name>`, `/write <text>`,
  `/copy [text]`, `/paste [text]`.
- `/playsound <file> …` / `/playlocalsound <file> …` (file, volume,
  ear-monitor, local-only).
- `/spotify …`, `/obs …`, `/colorpicker …` — sub-dispatchers.
- `/openfolder|/opendir|/openfile|/start <path-or-url>`.
- `/exec …`, `/batch …` — inline code, uploaded file, or file path.
- `/<plugin> …` — rhai plugin commands, args split on `<|§|>`.
