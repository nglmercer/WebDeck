# Preliminary release preparation

The current version remains `2.0.0-alpha.1`. This work prepares artifacts and verification; it does not tag or publish a public release.

## Reproducible artifacts

```bash
npm ci --prefix frontend
cargo run --locked --bin package -- --dev
node tools/release/installers.mjs --dev
```

Run on each native OS. The packager creates a portable ZIP and SHA-256; the installer builder verifies that digest, confines extraction and creates:

| Native runner | Output | Toolchain |
| --- | --- | --- |
| Ubuntu | `.deb` plus checksum | Python 3, `dpkg-deb` |
| Windows | per-user `-setup.exe` plus checksum | Python, Inno Setup 6 (`WEBDECK_ISCC` may override its path) |
| macOS | `.dmg` containing an application bundle plus checksum | Python 3, `sips`, `iconutil`, `plutil`, `hdiutil` |

Omit both `--dev` flags for optimized release artifacts. The development and release archive names are intentionally distinct. Installers place user data outside their installation directories. Native Windows and macOS artifacts must be produced on their own runners; a Linux package does not establish their validity.

CI checks Rust, the UI and embedded commands on all three platforms, builds preliminary packages, verifies extracted portable UI and the installed payload (temporary silent Windows installation on disposable CI runners) and uploads artifacts/checksums plus failure traces. Linux additionally runs the Android/tablet Chromium, iPhone/iPad WebKit and desktop Firefox experience suite. Workflow dispatch and `codex/**` pushes support running the same workflow in an Actions-enabled secondary repository.

## Release gates

| Gate | Evidence required before claiming readiness |
| --- | --- |
| Source checks | Rust tests/Clippy, contracts, translations, Svelte/types, unit tests, bundle budget |
| Desktop UI | Chromium acceptance on Linux, Windows and macOS |
| Mobile engines | `npm run test:devices --prefix frontend` with installed Chromium, Firefox and WebKit |
| Native packaging | Extracted portable smoke on each native OS; installer build and OS format validation |
| Real hardware | Device and native-effect sign-off below; emulator results do not replace it |
| Distribution | Windows publisher signing and macOS Developer ID/notarization before trusted production distribution |
| Recovery | Temporary data backup, upgrade and rollback, and uninstall preserving personal data |

The CI workflow uses simulated desktop effects for UI regression checks. It separately tests embedded runtime commands and executable plugin fixtures without an external JavaScript runtime. Actual keyboard, clipboard, media, capture, tray and account integrations need the disposable live fixtures in [manual verification](../manual-verification.md).

## Physical-device sign-off

Record OS/browser versions, build SHA, viewport, result and any screenshots. Run on at least one Android phone, one iPhone and one tablet on the local test network:

1. Pair a temporary device with limited capabilities and verify revocation.
2. Open every folder, scroll to the last row, rotate portrait/landscape and verify no horizontal overflow or missing cells.
3. Read long labels and measurements, open controls with a touch hold and confirm the held command is not executed.
4. Enter editing; Save/Done stay visible. Open and scroll Folder tools without moving deck cells.
5. Open the button editor, switch tabs, type with the virtual keyboard and dismiss it. Keep editing and Escape/cancel must retain text; deliberate discard must leave the saved host unchanged.
6. Apply changes, switch folders and settings, then save. Use a second temporary browser to cause a revision conflict; retain the draft and download a backup.
7. On Windows/macOS hosts, run the harmless real-effect fixtures from the manual-verification guide, then restore clipboard, volume, media and integration state.

At implementation time this workspace provides Linux/Chromium and emulated Android and tablet. Physical phones, Safari/WebKit/Firefox executables and native Windows/macOS are not available here. Their checks are prepared in CI and this sign-off guide; they are not recorded as passed until actual evidence exists.

See [the user quickstart](../quickstart-v2.md) and [the recording guide](../../examples/demo-v2/README.md).
