# Reference

The authoritative data/API types are [contracts/v2.schema.json](../contracts/v2.schema.json). Generated Rust and TypeScript contracts and the action catalog must stay synchronized using `node tools/contracts/generate.mjs --check`.

See [server routes](server.md), [typed commands](commands.md), [configuration](state-config.md) and [action inventory](v2/ACTION_INVENTORY.md).

Run `cargo run -- --help`, `cargo run --bin update -- --help` or `cargo run --bin webdeck-qr -- --help` for current CLI usage. The console reads typed command JSON from standard input. The app defaults to loopback port 5000 and `.config`. Use `--no-tray`, `--host`, `--port` and `--config-dir` to control startup. `WEBDECK_CONFIG_DIR` selects data, while console pairing uses `WEBDECK_URL` and `WEBDECK_DEVICE_TOKEN`.
