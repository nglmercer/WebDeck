# Rust migration (1:1)

This document tracks the Python → Rust migration of WebDeck. The migration is
**1:1 by module**: every `app/<path>/<module>.py` maps to
`src/app/<path>/<module>.rs`, and each `__init__.py` maps to its parent
`mod.rs`. Entry points map as `run.py` → `src/main.rs` (`webdeck` binary),
`console.py` → `src/bin/console.rs`, `app/updater/updater.py` (`__main__`
block) → `src/bin/update.rs`.

`src/lib.rs` is the only file without a Python counterpart: it exposes `app/`
so the three binaries share one implementation.

## Status legend

- **ported** — logic ported 1:1, covered by unit tests where pure.
- **routed** — dispatch/routing ported 1:1; leaf actions stubbed pending a
  platform crate (each stub names its planned crate).
- **stub** — signatures ported; body deferred (GUI, wire protocols, OS APIs).

## Module map

| Python | Rust | Status | Notes |
|---|---|---|---|
| `run.py` | `src/main.rs` | ported | server + tray spawn; UAC via `windows` crate |
| `console.py` | `src/bin/console.rs` | ported | REPL via `reqwest` |
| `app/server.py` | `src/app/server.rs` | ported | all routes + middleware; `socketioxide` layer; `/` renders via minijinja compat shims |
| `app/tray.py` | `src/app/tray.rs` | ported | `tray-icon` menu + `tao`/`wry` QR/config/port windows (Windows-only, like Python) |
| `app/buttons/commands.py` | `…/buttons/commands.rs` | ported | full dispatch incl. plugins; input/clipboard via `enigo`/`arboard` |
| `app/buttons/audio/*` (4) | `…/audio/*` | ported | CoreAudio via `windows` crate; media keys via `keybd_event` |
| `app/buttons/color_picker/*` (6) | `…/color_picker/*` | ported | capture via `screenshots`, clipboard via `arboard`, toast via `winrt-notification` |
| `app/buttons/exec/*` (4) | `…/exec/*` | ported | `/exec` scripts run as **rhai** (see deviations); `/batch` shells out; file-`/batch` mirrors upstream's no-op |
| `app/buttons/obs/*` (7) | `…/obs/*` | ported | wire protocol via `obws` (connect-per-command, same error mapping) |
| `app/buttons/soundboard/*` (5) | `…/soundboard/*` | ported | playback via `rodio`, mic loop via `cpal`, ffmpeg install/discovery + `apad`/`volume` filters |
| `app/buttons/spotify/*` (7) | `…/spotify/*` | ported | auth + API via `rspotify` (same redirect/scopes, `.cache-<user>` token file) |
| `app/buttons/system/*` (4) | `…/system/*` | ported | `openfile`/`opendir` real (explorer/xdg-open/open) |
| `app/buttons/usage/*` (3) | `…/usage/*` | ported | readings via `sysinfo`/`nvml-wrapper` |
| `app/buttons/window/*` (5) | `…/window/*` | ported | handles via `windows` crate |
| `app/on_start/*` (2) | `…/on_start/*` | ported | shortcuts via `windows` ShellLink, GPU probe via `nvml-wrapper`, VLC cache fix |
| `app/updater/*` (3) | `…/updater/*` | ported | `compare_versions`, `check_files`, download/extract (via `zip`), relaunch; UAC elevation |
| `app/utils/args.py` | `…/utils/args.rs` | ported | same flags via `clap`; `get_arg('x')` → `get_args().x` |
| `app/utils/debug/timers.py` | `…/utils/debug/timers.rs` | ported | `Instant`-based |
| `app/utils/exit.py` | `…/utils/exit.rs` | ported | `taskkill` approximation of WMI kill |
| `app/utils/firewall.py` | `…/utils/firewall.rs` | ported | `netsh` check instead of COM |
| `app/utils/get_local_ip.py` | `…/utils/get_local_ip.rs` | ported | same UDP trick, std-only |
| `app/utils/global_variables.py` | `…/utils/global_variables.rs` | ported | `Mutex<HashMap<String, Value>>` |
| `app/utils/is_opened.py` | `…/utils/is_opened.rs` | ported | `tasklist`/`/proc` scan |
| `app/utils/kill_nircmd.py` | `…/utils/kill_nircmd.rs` | ported | |
| `app/utils/languages.py` | `…/utils/languages.rs` | ported | same `.lang` format/rules |
| `app/utils/logger.py` | `…/utils/logger.rs` | ported | same files/levels/colors |
| `app/utils/merge_dicts.py` | `…/utils/merge_dicts.rs` | ported | |
| `app/utils/plugins/*` (2) | `…/utils/plugins/*` | ported | `.rhai` script plugins (see deviations); `.py` discovered-but-deferred |
| `app/utils/restart.py` | `…/utils/restart.rs` | ported | re-exec / respawn |
| `app/utils/settings/*` (6) | `…/utils/settings/*` | ported | incl. `audio_devices` via `cpal` |
| `app/utils/show_error.py` | `…/utils/show_error.rs` | ported | dialogs via `rfd` |
| `app/utils/themes/*` (2) | `…/utils/themes/*` | ported | |
| `app/utils/translate.py` | `…/utils/translate.rs` | ported | online endpoint via `reqwest` |
| `app/utils/welcome_popup.py` | `…/utils/welcome_popup.rs` | ported | `rfd` native dialog |
| `app/utils/working_dir.py` | `…/utils/working_dir.rs` | ported | `debug_assertions` ⇔ unfrozen |

## Crate equivalences

| Python | Rust | State |
|---|---|---|
| Flask | `axum` + `tower-http` | in use |
| Flask-SocketIO | `socketioxide` | in use |
| Jinja2 | `minijinja` | in use (compat shims at call site) |
| argparse | `clap` | in use |
| requests | `reqwest` (rustls) | in use |
| zipfile/tqdm | `zip` + logs | in use |
| colorama | inline ANSI | in use |
| pywin32/pycaw/comtypes | `windows` crate | in use |
| pyautogui/keyboard | `enigo` | in use |
| pyperclip | `arboard` | in use |
| pystray/tkinter/pywebview | `tray-icon`/`tao`/`wry` | in use (Windows) |
| easygui/tkinter dialogs | `rfd` | in use |
| qrcode/Pillow | `qrcode`/`image` | in use |
| mss | `screenshots` | in use |
| win10toast | `winrt-notification` | in use |
| psutil/GPUtil/pynvml | `sysinfo`/`nvml-wrapper` | in use |
| spotipy | `rspotify` (+`cli`) | in use |
| obs-websocket-py | `obws` | in use |
| python-vlc/nava/pyaudio/pydub | `rodio`/`cpal`/ffmpeg CLI | in use |
| deep_translator | `reqwest` web endpoint | in use |
| plugins (`importlib`) | `rhai` scripts + static registry | in use |
| `exec()` (buttons) | `rhai` sandbox | in use |
| cx_Freeze/setup.py | `cargo build --release` + packaging script | planned |
| VLC install (`fix_vlc_cache`) | registry check only | in use (no libvlc binding needed) |

## Build / run / test

```sh
cargo check --all-targets   # type-check incl. tests
cargo test                  # 56+ unit tests (pure ports)
cargo build --bins          # webdeck, console, update binaries
./target/debug/webdeck --no-tray -p 18080   # run server (dev: binds LAN IP)
./target/debug/webdeck --help               # clap flags (mirror args.py)
cargo run --bin console     # debug REPL (console.py)
cargo check --target x86_64-pc-windows-gnu --all-targets  # Windows coverage
```

## Intentional deviations (all documented at the call site)

- `serde_json` runs with `preserve_order`: Python dicts are
  insertion-ordered and `gridsize::unmatrix`/`save_single_button` depend on
  folder order; stock `serde_json` sorts keys.
- `get_arg('name')` → typed `get_args().name`; the `temp/webdeck_args.json`
  round-trip collapses to a static (it was in-process only).
- Plugin callables live in a static registry (`PluginFn`), not the
  `all_func` JSON global; `commands.rs` dispatches through it.
- `allowed_networks` uses CIDR-contains semantics; upstream's
  `remote_ip in ipaddress.ip_address(network)` raises `TypeError`.
- `sys.frozen` ⇔ release profile (`cfg!(debug_assertions)` is the dev check).
- `Logger.exception` renders `Debug` (`{:#?}`) instead of tracebacks.
- `get_system_language` parses `LC_ALL`/`LANG`/`LANGUAGE` (no locale API in std).
- `check_config_themes` parses stringified lists as JSON then Python-repr
  (no `eval`).
- `fix_firewall_permission`/`check_firewall_permission` use
  powershell/`netsh` instead of COM; `exit_program` uses `taskkill` instead
  of WMI (same observable effects).
- `/exec` scripts and `.rhai` plugins run **rhai**, not Python: embedding a
  Python interpreter would defeat the migration. Host API (`log_*`,
  `webdeck_command`, `run_shell`) is documented in `load_plugins.rs`.
  `.py` plugins are discovered but cannot be imported.
- Python raises inside button handlers map to `{"success": false,
  "message": …}` (exactly what `send_data_route` produces for raises);
  Python `None` returns map to `{"success": true}` (same route rule).
- Async service clients (`rspotify`, `obws`) are awaited through a
  throwaway current-thread runtime per call (button handling is sync).
- Soundboard: VLC/nava/pyaudio/pydub → `rodio` + `cpal` + ffmpeg CLI.
  Slot allocation, VB-Cable pairing, `fix_stop_soundboard` padding, and the
  `to_wav` cache behave as in Python; end-of-playback cleanup is a 500ms
  reaper instead of VLC events.
- Tray: pystray/tkinter/pywebview → `tray-icon` + `tao`/`wry`. Menu-item ids
  are stable dispatch keys; the integrated config window is maximized at
  creation; QR re-entry is guarded instead of lifting the old window.
- OBS: missing connection globals heal via `reload_obs()` instead of failing
  (fresh boot before any config save); the refused/password error predicates
  also match obws/tungstenite text. `/obs_start_virtualcam` is fixed: Python
  passes two args to one-arg `log.debug` and always crashes there.
- Spotify keeps the authenticated client instead of rebuilding from the
  token string per command (same observable behavior + free refresh).
- OBS `/obs_toggle_rec_pause` stays shadowed by `/obs_toggle_rec` (same
  branch order as Python).
- `TrayIcon` is `!Send`, so cross-thread menu updates go through polled
  desired-state instead of a shared icon handle.
- `rspotify` deprecated per-type library endpoints are called through the
  consolidated Library API (`library_add`/`remove`/`contains`) — same
  Spotify requests, no deprecated warnings.
- `cpal` 0.18 renamed `Device::name()` → `description().name()` and
  `SampleRate(u32)` → `u32`; error kinds via `Error::kind()`. `rodio`
  0.22 renamed `Sink` → `Player`, `OutputStream` → `MixerDeviceSink`.

## Known gaps / next steps

1. **Packaging**: release profile + installer/portable-zip script replacing
   `setup.py`/`build.bat`; Windows CI for `cfg(windows)` runtime coverage
   (compile coverage exists via the `x86_64-pc-windows-gnu` check).
2. **Native testing**: the Windows-only paths (tray, CoreAudio, `wry`
   windows, VB-Cable/rodio device selection) compile but need runs on a
   Windows host with OBS/Spotify/VLC-adjacent setups for end-to-end proof.

## Verification evidence (2026-09-26)

- `cargo check --all-targets`: clean, zero warnings (one pre-existing
  `screenshots` future-incompat note from the crate itself).
- `cargo check --target x86_64-pc-windows-gnu --all-targets`: clean, zero
  warnings — full Windows backend coverage including tray/rodio/cpal.
- `cargo test`: 56 passed, 0 failed, 1 ignored.
- Live smoke test (`--no-tray -p 18080`, isolated copy of
  `webdeck/`+`templates/`+`static/`): `POST /usage` → 200,
  `POST /send-data` (`/debug-send`, `/volume +`, `/exec type:single_line …`)
  → `{"success":true}`, `GET /get_config` → 200 real config,
  `/static/*` → 200, `POST /save_config` → `{"success":true}`,
  `GET /` → 200 rendered (minijinja compat shims).
- Repo tree untouched except additive `Cargo.toml`, `Cargo.lock`, `src/`,
  `docs/MIGRATION_RUST.md`, and a `target/` `.gitignore` entry; the Python
  app runs exactly as before.
