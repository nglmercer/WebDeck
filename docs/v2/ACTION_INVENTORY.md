# Canonical action inventory

`contracts/catalog.json` is generated from the same schema as both client/server
contracts; it contains each action's exact arguments and required capability.
There are no prefix aliases or text parsing rules.

| Capability | Actions |
| --- | --- |
| Read | debug, usage, configured button reference (resolved and checked against the actual action) |
| Input | key, write, copy, paste, cut, clipboard, clear clipboard, speech recognition |
| Audio | play/stop sound, system/app volume, mute, media transport, microphone/speakers |
| Window | foreground/close/kill/restart, desktop restart, open, color picker |
| Power | exit, shutdown, reboot, sleep, hibernate, lock, screensaver/settings |
| Script | explicit Rhai and shell sources |
| Network | fetch, OBS recording/stream/virtual camera/scene/hotkey, Spotify library/playback/playlist/follow/volume |
| Plugin | versioned plugin/action invocation, further narrowed by the manifest |
| Admin | firewall (local only; never grantable to a paired device) |
| Settings | configuration, uploads, catalogued device settings and editor operations |

Frontend actions are separately typed: folder navigation, reload, fullscreen,
settings and usage. They do not go through a native text-command dispatcher.
OS-specific availability and runtime validation limits are documented in `STATUS.md`.
