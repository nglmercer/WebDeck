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
