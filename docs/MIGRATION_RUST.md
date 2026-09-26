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
| `run.py` | `src/main.rs` | ported | UAC elevation TODO (`windows` crate) |
| `console.py` | `src/bin/console.rs` | ported | REPL via `reqwest` |
| `app/server.py` | `src/app/server.rs` | routed | all 13 routes + middleware; SocketIO TODO (`socketioxide`); `/` renders via minijinja, see gaps |
| `app/tray.py` | `src/app/tray.rs` | stub | config logic ported; icon/menu/windows TODO (`tray-icon` + `wry` + `rfd`) |
| `app/buttons/commands.py` | `…/buttons/commands.rs` | routed | full dispatch incl. plugins; input/clipboard TODO (`enigo`/`arboard`) |
| `app/buttons/audio/*` (4) | `…/audio/*` | routed | parsing ported; CoreAudio/keys TODO (`windows` crate) |
| `app/buttons/color_picker/*` (6) | `…/color_picker/*` | routed | `getarg`, `get_color_name`, handler logic ported; capture/clipboard/toast TODO |
| `app/buttons/exec/*` (4) | `…/exec/*` | routed | dispatch incl. threads ported; script exec TODO (`rhai`) |
| `app/buttons/obs/*` (7) | `…/obs/*` | routed | subcommand routing ported; wire TODO (`obws`) |
| `app/buttons/soundboard/*` (5) | `…/soundboard/*` | routed | `get_params`, vlc/nava dispatch, `replace_last_element` ported; audio TODO (`cpal`/`rodio`/`vlc`/`ffmpeg-sidecar`/`symphonia`) |
| `app/buttons/spotify/*` (7) | `…/spotify/*` | routed | token flow + routing ported; API TODO (`rspotify`) |
| `app/buttons/system/*` (4) | `…/system/*` | ported | `openfile`/`opendir` real (explorer/xdg-open/open) |
| `app/buttons/usage/*` (3) | `…/usage/*` | routed | gating + `asked_devices` ported; readings TODO (`sysinfo`/`nvml-wrapper`) |
| `app/buttons/window/*` (5) | `…/window/*` | stub | predicate ported; handles TODO (`windows` crate) |
| `app/on_start/*` (2) | `…/on_start/*` | routed | flow + `color_distance` + colors sort ported; shortcut/nvml/VLC TODO |
| `app/updater/*` (3) | `…/updater/*` | ported | `compare_versions`, `check_files`, download/extract (via `zip`), relaunch ported; UAC TODO |
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
| `app/utils/plugins/*` (2) | `…/utils/plugins/*` | routed | discovery ported; dynamic import → static registry + planned `libloading`/`rhai` |
| `app/utils/restart.py` | `…/utils/restart.rs` | ported | re-exec / respawn |
| `app/utils/settings/*` (6) | `…/utils/settings/*` | ported | except `audio_devices` (TODO `cpal`) |
| `app/utils/show_error.py` | `…/utils/show_error.rs` | routed | logging ported; dialogs TODO (`rfd`) |
| `app/utils/themes/*` (2) | `…/utils/themes/*` | ported | |
| `app/utils/translate.py` | `…/utils/translate.rs` | routed | EN path ported; online TODO (`reqwest`) |
| `app/utils/welcome_popup.py` | `…/utils/welcome_popup.rs` | stub | gate ported; window TODO |
| `app/utils/working_dir.py` | `…/utils/working_dir.rs` | ported | `debug_assertions` ⇔ unfrozen |

## Crate equivalences

| Python | Rust | State |
|---|---|---|
| Flask | `axum` + `tower-http` | in use |
| Flask-SocketIO | `socketioxide` | planned |
| Jinja2 | `minijinja` | in use (see gaps) |
| argparse | `clap` | in use |
| requests | `reqwest` (rustls) | in use |
| zipfile/tqdm | `zip` + logs | in use |
| colorama | inline ANSI | in use |
| pywin32/pycaw/comtypes | `windows` crate | planned |
| pyautogui/keyboard | `enigo` | planned |
| pyperclip | `arboard` | planned |
| pystray/tkinter/customtkinter | `tray-icon`/`wry`/`rfd` | planned |
| easygui | `rfd` | planned |
| qrcode/Pillow | `qrcode`/`image` | planned |
| mss | `screenshots`/`xcap` | planned |
| win10toast | `winrt-notification`/`notify-rust` | planned |
| psutil/GPUtil/pynvml | `sysinfo`/`nvml-wrapper` | planned |
| spotipy | `rspotify` | planned |
| obs-websocket-py | `obws` | planned |
| python-vlc/nava/pyaudio/pydub | `vlc`/`rodio`/`cpal`/`symphonia`/`ffmpeg-sidecar` | planned |
| deep_translator | `reqwest` web endpoint | planned |
| plugins (`importlib`) | static registry → `libloading`/`rhai` | planned |
| `exec()` (buttons) | `rhai` sandbox | planned |
| cx_Freeze/setup.py | `cargo build --release` + packaging script | planned |

## Build / run / test

```sh
cargo check --all-targets   # type-check incl. tests
cargo test                  # 27 unit tests (pure ports)
cargo build --bins          # webdeck, console, update binaries
./target/debug/webdeck --no-tray -p 18080   # run server (dev: binds LAN IP)
./target/debug/webdeck --help               # clap flags (mirror args.py)
cargo run --bin console     # debug REPL (console.py)
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

## Known gaps / next steps

1. **Template sandbox adaptation** (`templates/*.jinja`, esp. `index.jinja`).
   The templates use Python builtins (`eval`, `open`, `int/str/dict/type`)
   that minijinja intentionally withholds, plus Jinja2-only tolerance the
   engine lacks: `index.jinja:12` slices `config['front']['themes'][::-1]`,
   which **panics minijinja 2.24 on an empty list** (index OOB in
   `value/ops.rs`) instead of returning empty. `GET /` therefore returns a
   graceful 500 (render is `catch_unwind`-contained; the server stays up)
   until templates are adapted to pre-computed context. The Python app is
   unaffected (templates untouched).
2. **SocketIO** (`socketioxide` layer: `connect` log, `send` relay,
   `message_from_socket` → `handle_command` → emit `json_data`).
3. **Platform backends** in dependency order: `windows` (tray-adjacent first:
   firewall-COM parity, elevation, window/audio APIs), `enigo`+`arboard`
   (unlocks most `commands.rs` actions), `sysinfo`, `rspotify`, `obws`,
   soundboard audio stack, `rfd` dialogs, `tray-icon`+`wry`, `qrcode`+`image`,
   screen capture, `rhai` scriptexec/plugins, translator endpoint.
4. **Packaging**: release profile + installer/portable-zip script replacing
   `setup.py`/`build.bat`; Windows CI for `cfg(windows)` coverage.

## Verification evidence (2026-09-26)

- `cargo check --all-targets`: clean, zero warnings.
- `cargo test`: 27 passed, 0 failed.
- Live smoke test (`--no-tray -p 18080`, isolated copy of
  `webdeck/`+`templates/`+`static/`): `POST /usage` → 200,
  `POST /send-data` (`/debug-send`, `/volume +`) → `{"success":true}`,
  `GET /get_config` → 200 real config, `/static/*` → 200,
  `POST /save_config` → `{"success":true}`, `GET /` → graceful 500 with
  JSON error (minijinja gap above), server stays up for later requests.
- Repo tree untouched except additive `Cargo.toml`, `Cargo.lock`, `src/`,
  `docs/MIGRATION_RUST.md`, and a `target/` `.gitignore` entry; the Python
  app runs exactly as before.
