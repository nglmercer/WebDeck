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

Connect your phone from the computer with **Connect Device**. Scan the QR,
compare the verification code, and approve the device. No IP, port, or token entry
is needed. Choose temporary access, 30 days, or until revoked on the computer.
Returning devices restore their saved access on the same browser origin.
Controllers invoke button references; the server resolves and authorizes each
request, including nested actions and plugin permissions.

The host view shows the button grid and Connect Device. Usage readings live in buttons, folders
open as pages, and settings/edit shortcuts can be removed like any other button.
Right-click or hold the deck for controls, `Q` to edit, or `Ctrl+,` for settings.

## Phone access and hot reload

1. Open WebDeck on your computer.
2. Click **Connect Device**.
3. Scan the QR with your phone or tablet.
4. Compare the code and approve your phone on the computer.
5. Start controlling your deck.

**Settings → Devices & Connections** shows approved devices and their current
online status. **Manage device** renames, changes permissions, or renews access
without replacing the credential. Revoke access there at any time. Phones can
request additional permissions; the computer must explicitly approve them, and
the phone must attempt the action again.

Starting the wizard enables LAN access. WebDeck prefers a local network interface,
reuses a running listener, and picks an available port if the preferred one is
busy. Invitations expire after two minutes and can be used by one device. The QR
contains an invitation, never a device credential. A successful listener bind is
not proof that your phone can reach it.

Use a trusted local network: plain HTTP does not protect credentials from network
observers. Do not forward WebDeck ports to the internet. A separately configured
HTTPS reverse proxy must preserve Host/Origin validation and the localhost admin
boundary; it must never make forwarded clients appear local. See
[connection architecture and API](docs/v2/CONNECTIONS.md).

### Advanced network configuration

Expand **Advanced connection** for a specific address/port, listener disable,
manual token pairing, and HTTP/realtime transport selection. Check that the
computer and phone use the same network, allow the TCP port in the host firewall,
and disable router client isolation if needed. VPN and virtual interfaces are
ranked after ordinary network interfaces; choose an override when appropriate.
IPv6 link-local addresses are excluded because browser URLs need interface scopes.
Automatic exposure chooses private IPv4 or unique-local IPv6 addresses.

Network settings remain separate in `<config-dir>/network.v2.json`; deck backup
restores cannot enable LAN exposure. Failed reconfiguration retains the working
listener. The wizard detects address changes during this run and refreshes its
connection status. If the origin changes, scan a fresh QR and approve again:
browser storage does not transfer between addresses or ports. CLI flags `--lan`,
`--host`, and `--qr-url` remain available for advanced setup. The tray QR continues
to provide a plain deck URL; use the connection hub for invitation pairing.

For frontend development with hot reload, keep the Rust server running locally
and run this from the repository root:

```sh
npm run dev --prefix frontend -- --host 0.0.0.0
```

Open `http://YOUR_LAN_IP:5173/` on the phone (use the port printed by Vite).
Vite proxies API and Socket.IO requests to the Rust server on port 5000 and reloads
frontend source changes automatically. Pair the phone separately on this origin.
To show its QR without restarting the server:

```sh
./target/release/webdeck-qr --text http://YOUR_LAN_IP:5173/
```

Frontend hot reload is for source development. To pick up deck changes made in
another browser, use the deck's Reload control. Rust changes require rebuilding
and restarting the server.

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

The v2 deck adapts its visual orientation to portrait screens without rewriting saved cell positions. Q opens a fixed editing dock: tiles and labels keep their coordinates. Button settings are organized into Content, Appearance and Action tabs; host settings and the controls dialog also support keyboard-accessible tabs. Built-in icons use `icon:folder`, `icon:settings`, `icon:play`, and other SVG names available in the button icon picker. Uploaded `asset:` images remain supported.

The frontend JavaScript budget is 300 KB uncompressed / 91 KB gzip. This includes the inline SVG icon library, icon chooser and accessible tab panels; the demo no longer requires raster files for its built-in icons.

Start with [the v2 quickstart](docs/quickstart-v2.md). Preliminary native installers and remaining hardware checks are described in [release preparation](docs/v2/preliminary-release.md).

## Documentation guide

- [Quickstart](docs/quickstart-v2.md): first launch and basic deck editing.
- [CLI reference](docs/cli.md): manage the running deck, buttons and integrations.
- [Automation](docs/automation.md): generate buttons from recipes and use plugin actions.
- [Build and release](docs/build-release.md): build and package the application.
- [Contributing](CONTRIBUTING.md): development workflow and validation.
- [v2 documentation](docs/v2/README.md): architecture, migration and implementation status.

### Keyboard shortcuts and OBS projects

The key editor includes a searchable catalog and focused shortcut recording.
See [OBS projects](docs/obs-project.md) for declarative scenes, nested overlays,
managed scene buttons and recovery, and [LLM automation](docs/llm-automation.md)
for noninteractive CLI workflows.
