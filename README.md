# WebDeck

[![GitHub release](https://img.shields.io/github/v/release/Lenochxd/WebDeck.svg?style=flat)](https://github.com/Lenochxd/WebDeck/releases)
[![GitHub downloads](https://img.shields.io/github/downloads/Lenochxd/WebDeck/total.svg?style=flat)](https://github.com/Lenochxd/WebDeck/releases)
[![GitHub stars](https://img.shields.io/github/stars/Lenochxd/WebDeck.svg?style=flat)](https://github.com/Lenochxd/WebDeck/stargazers)
[![GitHub issues](https://img.shields.io/github/issues/Lenochxd/WebDeck.svg?style=flat)](https://github.com/Lenochxd/WebDeck/issues)
[![License: GPL-3.0](https://img.shields.io/badge/License-GPL--3.0-blue.svg?style=flat)](LICENSE)

WebDeck turns any browser on your local network — phone, tablet, second PC —
into a macro deck for your computer. No physical hardware needed (unlike an
Elgato StreamDeck): you host a lightweight server, open the page, and tap
buttons to control input, audio, windows, OBS, Spotify, scripts, system power,
and more, with live CPU/RAM/GPU/disk tiles streaming back to the page.

This is the Rust port of the original Python app (axum + Svelte SPA).

The `v2` integration branch adds typed command execution, configuration revisions,
pairing and feature-owned frontend state. See [implementation status and migration](docs/v2/STATUS.md)
for validation evidence and the remaining native release gates. Stable releases
and update channels are unchanged.

<img width="960" height="540" alt="3d-tour" src="https://github.com/user-attachments/assets/77972bde-4662-4125-80be-3f8dff45234e" />

## Quickstart

Download the latest `WebDeck-<os>-<arch>-portable.zip` from
[Releases](https://github.com/Lenochxd/WebDeck/releases), extract it, and run
`WebDeck` (`WebDeck.exe` on Windows). Then open the printed address
(default `http://<local-ip>:5000`) on any device in your network — scan the QR
popup with your phone for the fastest route.

Your config is created on first run (`.config/config.json`); edit buttons and
settings from the page itself (config view) or the tray icon.

## Run from source

Prerequisites: Rust toolchain, Node.js.

```sh
# 1. Build the frontend bundle (served by the backend)
cd frontend && npm ci && npm run build && cd ..

# 2. Run the app
cargo run
```

Common flags (`cargo run -- --help` for all): `--port`, `--host`,
`--no-tray` (console mode, Ctrl+C to stop), `--force-start`, `--no-admin`,
`--force-update` / `--no-auto-update`.

## Features

- Button grid with folders, custom images, backgrounds, and CSS themes
- Commands: keys/hotkeys, clipboard, app + system volume, media keys, window
  control, power actions, screensaver/lock, file/URL openers
- Integrations: OBS (scenes, streaming, recording, virtualcam), Spotify
  playback, soundboard, system usage tiles, Python/batch script exec
- rhai plugins (`.config/plugins/`) extending the button catalog and commands
- Live updates over HTTP + Socket.IO; LAN-only access guard with
  `allowed_networks` CIDR allowlist
- Auto-updater (portable zip), tray icon, QR popup, multi-language UI

## Documentation

Full docs live in [`docs/`](docs/index.md):

| Doc | Contents |
| --- | --- |
| [index](docs/index.md) | Overview, quickstart, conventions |
| [architecture](docs/architecture.md) | Binaries, lib layout, backend/frontend split |
| [startup](docs/startup.md) | Entry flow, CLI, on_start, tray, popup |
| [server](docs/server.md) | Routes, middleware, realtime, assets |
| [commands](docs/commands.md) | Command dispatcher and prefix map |
| [integrations](docs/integrations.md) | OBS, Spotify, soundboard, audio, exec, plugins |
| [state-config](docs/state-config.md) | Config files, save/migration, themes, languages |
| [frontend-shell](docs/frontend-shell.md) | Boot, grid, shell, usage loop |
| [frontend-editor](docs/frontend-editor.md) | Edit/add modals, arg schema, save flows |
| [build-release](docs/build-release.md) | Bins, packaging, updater |
| [reference](docs/reference.md) | HTTP + Socket.IO API, CLI, config schema |

## Contributing

Issues and PRs welcome on
[GitHub](https://github.com/Lenochxd/WebDeck). Dev loop:

```sh
cargo test && cargo clippy --all-targets   # backend
cd frontend && npm test && npm run typecheck && npm run build
```

## License

GPL-3.0-or-later — see [LICENSE](LICENSE).
