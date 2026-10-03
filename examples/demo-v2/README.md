# Master-layout demo with v2 actions

This recreates master's 8 × 4 demo: the same icons, tile positions, blank cells,
Folder 1 and Spotify navigation. All actions are canonical typed v2 actions;
settings and editor dialogs use the current v2 interface.

From the repository root:

```sh
npm ci --prefix frontend
node tools/demo/run.mjs
```

The launcher rebuilds the frontend and Rust debug server, creates a temporary
configuration, and cleans it up on exit. It never overwrites your `.config`.
Open http://127.0.0.1:5000. Set `WEBDECK_DEMO_PORT` to use another port.
Desktop effects are simulated by the debug-only fake platform; usage readings
come from the actual host, and unsupported GPU readings remain unavailable.
Press Q to edit, F1 for shortcuts, and right-click or hold for deck controls.

## Playwright video and screenshots

Install the browser and Playwright's video encoder once:

```bash
cd frontend
npm ci
npx playwright install chromium ffmpeg
cd ..
npm run demo:capture --prefix frontend
```

The capture command builds the frontend and Rust executable, starts an isolated demo server with simulated desktop effects, and records a desktop tour plus a mobile tour. It captures the home deck, media navigation, Q editing, all button-editor tabs, all settings tabs, shortcuts, and portrait/landscape layouts. It checks for browser errors and horizontal overflow during capture, then stops the server and removes the temporary configuration.

Output is written to `dist/demo-media/`: PNG screenshots, Playwright WebM recordings, replayable trace ZIPs and `manifest.json`. If a system `ffmpeg` is available, MP4 versions are exported too. No personal configuration is overwritten. Use `WEBDECK_DEMO_OUTPUT` to choose an output directory, `WEBDECK_DEMO_PORT` to change the server port, and `WEBDECK_CHROMIUM` to choose a browser executable. After a current build, `npm run demo:capture --prefix frontend -- --skip-build` skips rebuilding.

Open a trace with `npx playwright show-trace ../dist/demo-media/desktop-trace.zip` from `frontend/`.
