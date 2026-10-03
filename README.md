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

Rhai scripts use structured `invoke(#{type: "debug", data: #{value: 42}})` calls.
Plugins require a versioned v2 JSON manifest and a Rhai entry point; copy the
[example plugin](examples/plugins/echo.json) and its `.rhai` file into the selected
data directory's `plugins` folder, then restart. Python plugins and command-prefix
registries are unsupported. Plugins execute with the intersection of their
manifest capabilities and the caller's capabilities.

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
