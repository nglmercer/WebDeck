# Startup

Entry: `src/main.rs` (`#[tokio::main]`), port of `run.py`.

## Sequence

1. `working_dir::chdir_base()` — cd to the app base dir so relative paths
   (`webdeck/`, `.config/`, `static/`, `frontend/dist/`) resolve.
2. `args::parse_args()` — parse CLI (see below); handles `--version`,
   `--timeout`, `--log-file`, dev flags, and the `exit` positional.
3. `get_config(true, true)` — ensure config exists, migrate, save; read
   `settings.language` for translations.
4. Windows only: UAC self-elevation via `runas` when `settings.app_admin`
   is set, the process is not elevated, and `--no-admin` was not passed.
5. Single-instance guard: start only if `is_opened()` is false or
   `--force-start` was passed.
6. `languages::init("webdeck/translations", Some(".../misc"), &lang)`.
7. Spawn server task (`app::server::run_server()`) + blocking welcome popup.
8. Tray: `tray::create_tray_icon()` on a blocking thread (like
   `pystray.Icon.run()`); with `--no-tray`, wait for Ctrl+C instead.
9. Await server + popup handles on shutdown.

A failed server task shows a native error dialog off the async worker.

## CLI (`src/app/utils/args.rs`)

clap port of `argparse` flags (`Args` struct; `get_args()` replaces
`get_arg('…')`):

`-v/--version`, `-p/--port <u16>`, `-H/--host <addr>`, `-t/--timeout <secs>`,
`--no-admin` (alias `--no-sudo`), `--no-tray`, `--no-debug`,
`--force-start`, `--log-file <path>`, `--force-update` (alias `--update`),
`--no-auto-update` (alias `--no-update`), dev-only `--fake-error`,
`--test-ffmpeg` (rejected in release builds), and the `exit` positional
(`exit|stop|close|quit|kill|terminate|shutdown` stops all instances).

Notes:

- Parse errors exit with code 2 (`--help` exits 0), like argparse.
- Python persisted args to `temp/webdeck_args.json`; Rust keeps them in a
  static (`save_args`/`load_args`/`clear_args` are shims). `clear_args`
  still removes a stale Python temp file.
- `raw_args()` returns sorted, filtered argv (upstream quirk preserved).
- `--version` prints `WebDeck v<version.json> (<git-sha>)`; the sha suffix
  appears only in debug builds inside a git checkout.

## `on_start` (`src/app/on_start/utils.rs`)

`run_server()` calls `on_start()` first; it returns
`(config, commands, local_ip)`:

- Start-menu shortcut when `settings.windows_start_menu_shortcut` (Windows:
  `.lnk` via `IShellLinkW`; Linux: `~/.local/share/applications/WebDeck.desktop`,
  release builds only).
- Create `.config/user_uploads`, `.config/themes`, `.config/plugins`.
- Migrate legacy `static/files/uploaded/*` → `.config/user_uploads/`.
- `check_files()` (updater file reconciliation).
- `get_gpu_method()` — default `nvidia (NVML)`, NVML probe failure → `AMD`;
  a stuck `"None"` is re-probed (NVML → amdgpu → stay `None`) and saved.
- `check_for_updates()` when `(auto_updates || --force-update) && !--no-auto-update`.
- Load `webdeck/commands.json` + rhai plugins; store loaded plugin names in
  the `all_func` global (callables live in a static registry in Rust).
- Resolve local IP; if `url.ip == "local_ip"`, write the detected IP back.
- `on_start_threaded()`: background task sorting `webdeck/colors.json`
  (nearest-neighbor) when `settings.sort_colors_on_startup`, with a Gist
  fallback fetch if the file is missing/corrupt.

Then `run_server()` starts the soundboard mic loop if enabled, applies the
automatic firewall bypass if configured and missing, logs the local IP, marks
the tray server state `Running`, binds, and serves.

## Tray, popup, QR

- Tray (`src/app/tray/`): icon + menu + state; language switch via
  `change_tray_language`; server state via `change_server_state`.
- Welcome popup (`utils/welcome_popup.rs`): blocking window shown at startup
  (spawned via `spawn_blocking`).
- QR (`src/bin/qr.rs`, `webdeck-qr` binary): minifb window rendering the
  server URL as a QR code for phones/tablets. On Wayland the child may log
  `queue 0x… destroyed while proxies still attached` — a harmless minifb
  teardown warning.
