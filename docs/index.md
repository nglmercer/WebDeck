# WebDeck (Rust) — Documentation

WebDeck turns any browser on your local network (phone, tablet, second PC)
into a hardware-style macro deck for your computer: buttons trigger commands
(input, audio, windows, OBS, Spotify, scripts, system power, …) while usage
tiles stream live CPU/RAM/GPU/disk stats back to the page.

This is the Rust port of the original Python app. Behavior is ported 1:1 where
possible; where the stacks differ (Flask → axum, Jinja → Svelte SPA,
cx_Freeze → cargo + portable zip) the Rust behavior documented here wins.

## Quickstart

Prerequisites: Rust toolchain, Node.js (for the frontend bundle).

```sh
# 1. Build the frontend bundle (served by GET /)
cd frontend && npm ci && npm run build && cd ..

# 2. Run the app (creates .config/config.json on first run)
cargo run

# 3. Open the printed address (default http://<local-ip>:5000)
```

Useful flags: `cargo run -- --help`. Common ones: `--port`, `--host`,
`--no-tray` (console mode, Ctrl+C to stop), `--force-start`, `--no-admin`,
`--no-debug`, `--log-file`, `--force-update` / `--no-auto-update`,
plus the `exit|stop|close|quit|kill|terminate|shutdown` positional to stop
all running instances.

## Map

| Doc | Contents |
| --- | --- |
| [architecture](architecture.md) | Binaries, lib layout, backend/frontend split |
| [startup](startup.md) | Entry flow, CLI, working dir, on_start, tray, popup |
| [server](server.md) | axum routes, middleware, realtime,assets |
| [commands](commands.md) | `/send-data` dispatcher and command prefixes |
| [integrations](integrations.md) | OBS, Spotify, soundboard, audio, exec, plugins |
| [state-config](state-config.md) | Config files, atomic save, migration, themes, languages |
| [frontend-shell](frontend-shell.md) | Boot, grid, shell, state, usage loop |
| [frontend-editor](frontend-editor.md) | Edit/add modals, arg schema, buildCommand protocol |
| [build-release](build-release.md) | Bins, packaging, updater, version.json |
| [reference](reference.md) | HTTP + Socket.IO API, CLI, config schema |
| [refactor](refactor.md) | Cleanup phases, removed code, gates, remaining work |

## Conventions used in these docs

- `message` means the command string a button sends (e.g. `/volume set 50`).
- `<|§|>` inside a message is normalized to a space before dispatch.
- Paths like `src/app/server/mod.rs` are relative to the repo root.
- "Python did X" notes explain a port decision; they are not a spec.
