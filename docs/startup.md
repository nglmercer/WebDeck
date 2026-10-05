# Startup

`src/main.rs` selects `--config-dir PATH`, then `WEBDECK_CONFIG_DIR`, then `.config`. It opens canonical configuration and device sessions, initializes native integrations and plugins, creates the executor and binds the server. The default listener is `127.0.0.1:5000`; `--host` and `--port` override it.

A missing configuration is initialized from `webdeck/config_default.json`, unless an existing root `config.json` requires manual recovery. Incompatible documents stop startup and remain unchanged. An old `front` layout is unsupported even when marked as schema version 2. Move it to a backup or choose an empty config directory before restarting with defaults.

`--no-tray` runs the server without its native tray. Otherwise the tray runs on its own thread and can open the browser or QR viewer. Ctrl+C or tray shutdown stops admission and drains accepted work. Run `cargo run -- --help` for parsed options. Automatic updates remain disabled.
