# Startup

`src/main.rs` selects `--config-dir PATH`, then `WEBDECK_CONFIG_DIR`, then `.config`. It opens canonical configuration and device sessions, initializes native integrations and plugins, creates the executor and binds the server. The default listener is `127.0.0.1:5000`; `--host` and `--port` override it.

A missing configuration is initialized from `webdeck/config_default.json`, unless an existing root `config.json` requires manual recovery. Incompatible documents stop startup and remain unchanged. An old `front` layout is unsupported even when marked as schema version 2. Move it to a backup or choose an empty config directory before restarting with defaults.

`--no-tray` runs the server without its native tray. Otherwise the tray runs on its own thread and can open the browser or QR viewer. Ctrl+C or tray shutdown stops admission and drains accepted work. Run `cargo run -- --help` for parsed options. Automatic updates remain disabled.

## Windows background mode and tray

`webdeck.exe` and `webdeck-qr.exe` use the Windows GUI subsystem, so launching
from Explorer does not create a console window. The main app starts its server
and tray without opening another application window. The `console`, `webdeckctl`,
`update` and packaging tools remain console applications.

The native tray menu shows that WebDeck is running and provides Open WebDeck,
Settings, Devices & Connections, Help and tutorials, Show QR code, and Exit.
Left click opens the browser; right click opens the native menu. The tooltip
includes the local server URL. Menu text and status use the same seven translated
dictionaries as the web interface and refresh when the selected language changes.
Startup and tray errors use a native Windows error dialog. If the required tray
cannot be created, WebDeck shuts down instead of leaving an inaccessible hidden
server. Internal shell/system helpers suppress Windows console flashes; explicitly
launched application processes retain their own window behavior.

A native icon and context menu are enough for the tray. The existing browser UI
renders settings and tutorials; no HTML tray window or additional WebView runtime
is required. See the [Rust GUI subsystem attribute](https://doc.rust-lang.org/reference/runtime.html#the-windows_subsystem-attribute)
and [tray menu options](https://docs.rs/tray-icon/0.25.1/tray_icon/struct.TrayIconBuilder.html#method.with_menu_on_left_click).

## Direct browser links

The following links use the default local port; change the address/port to match
your listener. They navigate only and do not change configuration or start LAN
access:

| Destination | Link |
| --- | --- |
| Deck | `http://127.0.0.1:5000/` |
| Settings | `http://127.0.0.1:5000/?view=settings` |
| Devices & Connections | `http://127.0.0.1:5000/?view=settings&section=devices` |
| Help and tutorials | `http://127.0.0.1:5000/?view=settings&section=tutorials` |

Other supported settings sections are `appearance`, `integrations`, `backups`
and `runtime`. The tutorial link opens the existing learning library over
Settings after authorization and translation loading. Controllers without local
editing permission stay on the deck. Unknown sections fall back to Appearance.
The existing `#settings` and `#folder=...` links still work; folder hashes take
precedence over settings query parameters. Back to deck removes only `view` and
`section` and preserves unrelated query parameters, so reload cannot reopen the
tutorial unexpectedly.

Windows CI checks actual PE headers for GUI subsystem executables and verifies
that `webdeckctl.exe` retains the console subsystem.
