# WebDeck v2: first deck in five minutes

This is a preliminary v2 build. Use the package for your operating system and CPU architecture, and retain its SHA-256 file. Development artifacts include `-dev` in their name; they are intended for evaluation.

## Install and start

- **Windows:** run the `-setup.exe` installer. It installs for your current account and creates a Start-menu shortcut. The shortcut stores personal data under `%LOCALAPPDATA%\WebDeck`.
- **Ubuntu/Debian:** run `sudo apt install ./WebDeck-…linux…deb`, then open WebDeck from the applications menu or run `webdeck`. Personal data lives under `$XDG_CONFIG_HOME/webdeck`, or `~/.config/webdeck`.
- **macOS:** open the `.dmg` and drag WebDeck into Applications. Personal data lives under `~/Library/Application Support/WebDeck`. Preliminary packages are unsigned and not notarized; signing and native verification remain release gates.
- **Portable ZIP:** extract the entire `WebDeck/` folder into a writable directory, open a terminal there, and launch `./WebDeck` or `WebDeck.exe`. Keep `frontend/`, `webdeck/`, and `static/` beside the executable. Do not run the binary directly out of the ZIP.

Open `http://127.0.0.1:5000/` on the host computer. If a port is busy, use `--port 5001`. The first deck contains media, settings, fullscreen and metric tiles. A missing GPU reading means no compatible metric provider is available.

## Build your deck

1. Click **Edit** or press **Q**. Tiles retain their positions.
2. On a touchscreen, tap a tile to edit it. On desktop, use its pencil control. Choose Content, Appearance or Action to edit a name, SVG icon, span or typed action.
3. Choose **Apply to draft**. This stages the button without immediately changing the host configuration.
4. Choose **Save changes** to persist the whole deck. **Done** also saves pending deck changes before leaving editing. A failed save or revision conflict retains your draft.
5. On mobile, expand **Folder tools** to rename or switch folders, add buttons/folders or open settings. Save and Done remain fixed outside that scrolling tools area.

Closing a changed button editor asks whether to keep editing or discard that button draft. It does not discard changes already applied to the deck. Going to another folder or Settings preserves the deck draft. Reloading an unsaved configuration asks separately before replacing it.

Touch decks scroll vertically so their labels and touch targets remain readable. Turn the device to change the visual orientation; saved cell coordinates are not rewritten. Use Settings tabs for appearance, integrations, devices, backups, runtime and connection.

## Pair a phone or tablet

Start the host with `--host 0.0.0.0` to listen on the local network. On the host browser, open Settings → Devices, name the temporary controller and grant only the capabilities it needs. Open `http://HOST_LOCAL_IP:5000/` on the device and enter its one-time pairing token. Use a trusted local network for this preliminary HTTP setup. Revoke temporary approvals when testing is complete; do not publish tokens in screenshots.

## Backups and upgrades

Settings → Backups downloads a canonical v2 backup. Keep uploads and the personal data directory with it. Older incompatible configurations are rejected; this build does not silently convert them.

For installed packages, quit WebDeck and upgrade with the OS package method. For portable packages, use the verified manual updater and its rollback command described in [build and release](build-release.md). Keep a known-good package and data backup. Automatic updates are disabled.

## Source checkout and demo

```bash
npm ci --prefix frontend
npm run build --prefix frontend
cargo run --locked --bin webdeck
```

Native build dependencies are in the repository README. `node tools/demo/run.mjs` launches an isolated master-style demo with simulated desktop effects. `npm run demo:capture --prefix frontend` creates Playwright videos, screenshots and traces. These recordings demonstrate UI behavior, not live keyboard/audio/OBS/Spotify effects.
