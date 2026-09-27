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
| `app/server.py` | `src/app/server.rs` | ported | all routes + middleware; `socketioxide` layer; `/` serves the TypeScript SPA bundle, `/api/boot` the ex-template context |
| `app/tray.py` | `src/app/tray.rs` | ported | `tray-icon` menu + `tao`/`wry` QR/config/port windows; Linux via `ksni` StatusNotifier |
| `app/buttons/commands.py` | `…/buttons/commands.rs` | ported | full dispatch incl. plugins; input/clipboard via `enigo`/`arboard` |
| `app/buttons/audio/*` (4) | `…/audio/*` | ported | CoreAudio via `windows` crate; Linux via `pactl`; media keys via `keybd_event` |
| `app/buttons/color_picker/*` (6) | `…/color_picker/*` | ported | capture via `screenshots` (+ `grim` fallback on Wayland), clipboard via `arboard`, toast via `winrt-notification`/`notify-rust` |
| `app/buttons/exec/*` (4) | `…/exec/*` | ported | `/exec` scripts run as **rhai** (see deviations); `/batch` shells out; file-`/batch` mirrors upstream's no-op |
| `app/buttons/obs/*` (7) | `…/obs/*` | ported | wire protocol via `obws` (connect-per-command, same error mapping) |
| `app/buttons/soundboard/*` (5) | `…/soundboard/*` | ported | playback via `rodio`, mic loop via `cpal`, ffmpeg install/discovery + `apad`/`volume` filters |
| `app/buttons/spotify/*` (7) | `…/spotify/*` | ported | auth + API via `rspotify` (same redirect/scopes, `.cache-<user>` token file) |
| `app/buttons/system/*` (4) | `…/system/*` | ported | `openfile`/`opendir` real (explorer/xdg-open/open) |
| `app/buttons/usage/*` (3) | `…/usage/*` | ported | readings via `sysinfo`/`nvml-wrapper` |
| `app/buttons/window/*` (5) | `…/window/*` | ported | handles via `windows` crate; Linux via `wmctrl`/`xdotool` (X11) |
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
| Jinja2 | TypeScript SPA (`frontend/`, custom framework) | templates deleted; `/api/boot` serves the ex-template context as JSON |
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
(cd frontend && npm install && npm run build)  # build TS web UI into frontend/dist (required for GET /)
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
- `/usage` percents are rounded to 1 decimal (psutil reports
  `round(x, 1)`); sysinfo math yields full precision.
- The tile-update loop in `index.jinja` contains per-tile `eval`
  failures (a missing path used to abort every later tile); shared
  with — and equally fixed for — the Python app.

## Linux backends (no Python required)

Every Windows-only API has a Linux equivalent behind `cfg(target_os =
"linux")`; dispatch signatures are unchanged:

- Audio (volume/mic/speaker/appvolume): `pactl` (PipeWire/PulseAudio).
- PC control: `systemctl`/`loginctl` (power, session, lock), screensaver via
  `qdbus`/GNOME/KDE calls, kill via `/proc` scan, media keys via MPRIS
  (`playerctl`/`qdbus`).
- Window management: `wmctrl` + `xdotool` (X11; native Wayland window
  control is a compositor limitation).
- Tray: `ksni` StatusNotifier menu; QR/config/port windows still `tao`/`wry`.
- Shortcuts/autostart: `.desktop` files
  (`~/.local/share/applications`, `~/.config/autostart`).
- ffmpeg: system `PATH` lookup (`apt install ffmpeg`); no download.
- Color picker: `screenshots` crate, with a `grim` 1x1 fallback on Wayland.
- Toasts: `notify-rust`; dialogs stay `rfd`.
- Disks: basename aliases alongside the 1:1 device-path keys
  (`/dev/nvme0n1p4` → `nvme0n1p4`), since paths are not valid JS eval
  paths for the usage tiles.
- AMD GPU (`gpu_method == "AMD"`): `gpu_busy_percent` /
  `gpu_metrics` / `mem_info_vram_*` sysfs reads (offsets per
  `kgd_pp_interface.h`, range-gated; missing counters are omitted,
  never zero-filled).
- GPU method resilience (deviation from Python, which bricked GPU tiles
  on any NVML failure): a failed NVML read no longer rewrites
  `gpu_method` to `"None"`; startup re-probes a stuck `"None"` (NVML →
  amdgpu → stay `"None"`); and the read path falls back to a working
  backend when the configured method is `"None"`/empty/unknown, so
  stale saves can't blank the tiles. Every method yields at least an
  (empty) `defaultGPU` key.

## Known gaps / next steps

1. **Windows CI / native testing**: the Windows-only paths (tray, CoreAudio,
   `wry` windows, VB-Cable/rodio device selection, MSI installer) compile
   but need runs on a Windows host for end-to-end proof. MSI generation
   (`setup.py bdist_msi`) stays a manual WiX step; the portable zip below
   is the shippable + auto-update artifact.
2. **Deliberate 1:1 deviations kept** (all documented at the call site):
   `force` exits terminate on Linux (Python no-ops; tray/updater only ran
   on Windows there), `.py` plugins are rejected in favor of `.rhai`
   scripts (executing Python would re-add a Python requirement), and the
   firewall check uses `netsh` instead of COM `HNetCfg.FwMgr`.

## Verification evidence (2026-09-26, updated 2026-09-27)

- `cargo check --all-targets`: clean, zero warnings (one pre-existing
  `screenshots` future-incompat note from the crate itself).
- `cargo check --target x86_64-pc-windows-gnu --all-targets`: clean, zero
  warnings — full Windows backend coverage including tray/rodio/cpal.
- `cargo test`: 77 passed (75 lib + 2 CLI), 0 failed, 1 ignored (incl.
  Linux pactl/wmctrl/grim-parser/backend unit tests; tray menu test inits
  real lang files; volume failures surface as `{"success": false}` like
  Python; usage-tile template chain + `/proc/net/dev` network-totals
  oracle; gpu_metrics fixture parse + live amdgpu shape +
  disk-alias/rounding checks; `exit` positional terminates; bad CLI args
  exit(2) like argparse).
- Full Python→Rust audit (2026-09-27): every `app/*.py` module has a
  same-named Rust counterpart; 51/51 button symbols and 83/83 command
  prefixes ported; all 15 server routes + socket.io namespace ported;
  tray menu/popup/console ported; no `todo!`/`unimplemented!` bodies.
  Gaps found and closed: argparse exit(2) + release dev-flag rejection,
  `.lang` error propagation, Windows locale API, local log timestamps,
  exception stack capture, console 200-exactness, and `src/bin/package.rs`
  (`setup.py`/`build.bat` replacement, verified end-to-end with a runnable
  portable zip — unlike `setup.py`, it ships `frontend/dist`, which the
  old ignore rules silently excluded and left the frozen web UI broken).
- Live smoke test (`--no-tray -p 18080`): `POST /usage` → 200,
  `POST /send-data` (`/debug-send`, `/volume +`, `/exec type:single_line …`)
  → `{"success":true}`, `GET /get_config` → 200 real config,
  `/static/*` → 200, `POST /save_config` → `{"success":true}`,
  `GET /` → 200 rendered TypeScript SPA.
- Python removal (2026-09-27): `app/`, `run.py`, `console.py`, `setup.py`,
  `requirements.txt`, and `build.bat` deleted after the audit + gap work
  above; the repo builds, tests, packages, and runs with no Python.
