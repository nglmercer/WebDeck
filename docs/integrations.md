# Integrations

`src/native.rs` dispatches typed commands to OS, audio, OBS, Spotify, HTTP and script implementations. Platform-specific helpers live under `src/native/`. Native availability depends on the desktop session, system libraries and installed tools; see [validation limits](v2/STATUS.md).

OBS settings contain host, integer port and password. Spotify settings contain client ID, client secret and redirect URI. Local administrators initiate Spotify connection through `/api/v2/spotify/connect`; tokens are stored in the selected config directory.

Sound playback uses explicit file sources, volume and output device. Uploads use protected opaque asset IDs; privileged external file references must be absolute paths.

Plugins use a versioned JSON manifest and Rhai entry point in the data directory's `plugins` folder. See [the example](../examples/plugins/echo.json). Restart after changing plugins. Their capabilities intersect the caller's grant. Rhai scripts use structured actions; shell scripts require explicit source and timeout.
