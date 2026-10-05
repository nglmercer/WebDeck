# Integrations

Commands run in the embedded JavaScript runtime. `src/capabilities/` supplies authorized OS, audio, bounded HTTP/WebSocket and storage primitives. JavaScript in `runtime/src/builtins/` implements OBS and Spotify protocols. Platform availability still depends on the desktop session, system libraries and installed tools; see [validation limits](v2/STATUS.md).

OBS settings contain host, integer port and password. Spotify settings contain client ID, client secret and redirect URI. Local administrators initiate Spotify connection through `/api/v2/spotify/connect`; tokens remain in the selected config directory. Only the captured trusted built-in integration handlers can obtain those credentials.

Sound playback uses explicit file sources, volume and output device. Uploads use protected opaque asset IDs; privileged external file references must be absolute paths.

Plugins use a versioned `webdeck.json` and verified JavaScript entry inside `<config-dir>/plugins/<id>/`. Copy the [echo package](../examples/plugins/echo), then reload from local Settings. Caller grants must include the Plugin capability and every capability declared by the action. During execution the grant narrows to that declaration. State is isolated per package, and failed packages can be reset by reloading.

The [runtime guide](v2/NAPI_VM_MIGRATION.md) documents both sandbox JavaScript and explicitly trusted native executable packages. Shell scripts retain explicit source and timeout. Earlier Rhai scripts require manual translation; files are never automatically converted.
