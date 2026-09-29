# Integrations

## OBS (`src/app/buttons/obs/`)

`command_handler` dispatches `/obs …` (scenes, streaming, recording,
virtualcam, hotkeys) via the `obws` client (OBS WebSocket, default
`localhost:4455` from `settings.obs`). `reload_obs()` reconnects after a
config save changed the OBS section — no restart needed. Streamlabs OBS is
not supported (same as Python).

## Spotify (`src/app/buttons/spotify/`)

`command_handler` dispatches `/spotify …` via `rspotify`: playback
(play/pause/next/previous), songs, albums, artists, playlists, volume.
Credentials live in `settings.spotify_api`
(`client_id`, `client_secret`, `username`). HTTP is blocking inside
`spawn_blocking` command handling, like the old gevent worker.

## Soundboard (`src/app/buttons/soundboard/`)

- `player` — play local files through rodio/cpal (`/playsound`,
  `/playlocalsound`, `/stop_sound`); params parsed by `get_params`.
- `mic` — optional mic loop (`start_if_enabled` at server start;
  `stop`/`restart` on config save). There is no VLC anymore, so the old
  `fix_vlc_cache` step is gone.
- `ffmpeg` — resolves an ffmpeg binary (transcode/probe helper);
  `--test-ffmpeg` prints the resolved path and exits.
- `devices`, `mic` — device enumeration feeding `/api/boot audio_devices`.

Config: `settings.soundboard` (`enabled`, `audio_method`, `mic_input_device`,
`vbcable`) plus `settings.ear_soundboard`, `settings.fix_stop_soundboard`.

## Audio (`src/app/buttons/audio/`)

- `volume` — `/volume …` parsing + system volume (`change_volume`).
- `set_system_mic` / `set_system_speaker` — `/setmicrophone`, `/setoutputdevice`.
- `app_volume` (under `buttons/commands/`) — `/appvolume` per-app control.
- `policy_config` — Windows audio-policy COM helpers.
- `settings/audio_devices.rs` — input/output device lists for boot.

## System open (`src/app/buttons/system/`)

`/openfolder`, `/opendir`, `/openfile`, `/start` → `handle_command`
(`opendir`, `openfile`): open files, folders, URLs with the OS default.

## Windows (`src/app/buttons/window/`)

`find_window` / `get_focused` / `focus_window` / `close_window`: lookup by
name, focused-handle query, foreground, close. Used by `/kill`-family,
`/superAltF4`, `/firstplan`, `/restartexplorer`.

## Script exec (`src/app/buttons/exec/`)

- `/exec …` → `python(...)`: run Python code, an uploaded file, or a file path.
- `/batch …` → `batch(...)`: run batch/shell code.
- Errors return `{success:false, message}` instead of success.

## HTTP fetch (`src/app/buttons/fetch.rs`)

`/fetch …` → `fetch(...)`: send an HTTP request to another app (webhook,
REST API) from a button. The `Integrations → Fetch URL` entry in
`webdeck/commands.json` renders the form (method dropdown, URL, a
name/value headers editor, body, timeout — the body hides for `GET`/`HEAD`
via `visibleWhen`); each field is preceded by a hidden `text` marker
(`method:`, `url:`, …) because the form drops empty values, so the
backend pairs markers — not positions — with values (same convention as
`/exec`'s `type:` markers). A manual `/fetch https://host/hook` form sends
a bare GET with defaults (10 s timeout).

Methods: GET, POST, PUT, PATCH, DELETE, HEAD, OPTIONS. Headers are
`Name: value` lines; the timeout clamps to 1–120 s. Transport errors and
non-2xx statuses return `{success:false}` (with `status` + truncated
`body` echoed back); 2xx returns `{success:true, status, body}`. Header
and body values are never logged, and response reads cap at 1 MiB.

## Plugins (rhai)

`src/app/utils/plugins/load_plugins.rs`: `.config/plugins/` scripts extend
the button catalog (`load_plugins(commands)` merges them in `on_start` and
`/api/boot`) and register commands in the `plugin_commands()` registry
(`PluginFn(&[String])` convention). Message args after `/<command> ` split
on `<|§|>`. The `all_func` global holds loaded plugin *names* (callables
live in a static registry — Rust has no Python-style func map).

## Notifications / clipboard / input

`notify-rust` (desktop notifications), `arboard` (clipboard),
`enigo` (keys/hotkeys/typing). Linux uses xdg portals (`rfd` file dialogs,
ashpd screenshot) and ksni tray — no libappindicator needed.
