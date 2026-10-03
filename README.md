# WebDeck v2

WebDeck turns a browser into a control deck for a Windows or Linux computer. This
`2.0.0-alpha.1` prerelease rebuilds the application around typed actions, stable
button and folder IDs, paired devices, revision-aware storage and a Svelte UI.

## Run from source

```sh
npm ci --prefix frontend
npm run build --prefix frontend
cargo run --locked --bin webdeck -- --no-tray
```

Rebuild the frontend after pulling changes. `cargo run` compiles Rust, not the
browser bundle. WebDeck checks the bundle's contract fingerprint and explains
how to rebuild an incompatible frontend instead of serving a blank deck.

To run the master-style 8 × 4 demo with v2 navigation and dialogs, run
`node tools/demo/run.mjs` after installing frontend dependencies. The launcher
builds both parts and uses a temporary configuration with simulated desktop
effects. See [the demo guide](examples/demo-v2/README.md).

Open `http://127.0.0.1:5000`. Run without `--no-tray` for the native tray and QR
viewer. Linux builds require GTK 3, ALSA, XCB and pkg-config development packages.
Audio and desktop actions also depend on the desktop session and installed tools
such as PipeWire/PulseAudio and wmctrl. Media controls use MPRIS directly.
On Wayland, keyboard actions request host approval through the desktop portal;
WebDeck retains that session while running.

For LAN access, start with `--host 0.0.0.0`. Approve a device in Settings from the
local browser, choose its capabilities and expiry, and enter its token on the
controller. Tokens never go in URLs. A controller cannot retrieve integration
credentials or the source of configured actions: it invokes button references,
and the server resolves and authorizes them on each request. Use a trusted network when transporting credentials.

The normal view is just the button grid. Usage readings live in buttons, folders
open as pages, and settings/edit shortcuts can be removed like any other button.
Right-click or hold the deck for controls, `Q` to edit, or `Ctrl+,` for settings.

## Configuration and backups

The default data directory is `.config`; use `--config-dir PATH` or
`WEBDECK_CONFIG_DIR` to select another directory. Existing data is never converted,
moved or deleted. Only the canonical document in
[`contracts/v2.schema.json`](contracts/v2.schema.json) is accepted. An older
configuration, including an older schema-2 document with text commands, stops
startup without rewriting it. Use an isolated directory to evaluate the rewrite.

Settings can download and restore canonical v2 backups. Restoration edits a draft;
saving still requires the revision loaded by the editor. A conflict preserves the
unsaved draft. Arrays define folder and button order, while stable IDs identify
individual items. Custom data belongs in explicit `extensions` objects.

## Commands and plugins

All clients use `/api/v2` and the `/v2` Socket.IO namespace. A command is a typed
JSON object, for example:

```json
{"request_id":"example-1","command":{"type":"key","keys":["ctrl","c"]}}
```

The console accepts one such command object per line (without the request wrapper).
Use `WEBDECK_URL` and `WEBDECK_DEVICE_TOKEN` for a paired console.

JavaScript scripts call structured actions with `ctx.invoke({type: "debug", data: {value: 42}})`.
The embedded napi-vm runtime requires no Node, Bun or npm installation.
Copy the [example plugin directory](examples/plugins/echo) to `<config-dir>/plugins/echo`.
Plugins use a v2 `webdeck.json` manifest, verified JavaScript source and declared capabilities.
The local Settings page can inspect and reload plugins. Executable native plugins use
napi-vm's trusted process host and require explicit installation; their processes have
ordinary OS privileges. See the [runtime migration guide](docs/v2/NAPI_VM_MIGRATION.md)
for workflows, package formats and manual migration of earlier scripts.

## Validation and packaging

See [CONTRIBUTING.md](CONTRIBUTING.md) for feature ownership, complete checks,
performance profiling, and redacted command diagnostics.

```sh
node tools/contracts/generate.mjs --check
node tools/validation/v2-only.mjs
cargo test --locked --all-targets
cargo clippy --locked --all-targets --all-features -- -D warnings
npm run check:components --prefix frontend
npm run typecheck --prefix frontend
npm test --prefix frontend
npm run test:acceptance --prefix frontend
cargo run --locked --bin package -- --dev
```

Install the component-check tool with `npm ci --prefix tools/component-check`
before checking Svelte. Browser acceptance tests require a built `webdeck` binary,
a built frontend and Playwright Chromium. They use isolated data and fake effects.
Development packages are labelled `-dev-portable.zip` and are not published by CI.

Automatic updates remain disabled. The explicit updater checks only verified,
newer v2 prereleases from `nglmercer/WebDeck`; installation verifies SHA-256,
confines extraction and retains a rollback journal. Run `update --help` for manual
check, fetch, install and rollback commands. Quit WebDeck before replacing binaries.

See [implementation and validation status](docs/v2/STATUS.md) for platform gaps and
[migration details](docs/v2/MIGRATION_REPORT.md) for the removed interfaces.
