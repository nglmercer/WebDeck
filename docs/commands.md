# Commands

> V2 implementation: see [architecture, compatibility and validation](v2/STATUS.md). The historical descriptions below document the v1 compatibility layer.

Dispatcher: `handle_command(message)` in `src/app/buttons/commands/mod.rs`,
called by `POST /send-data` and Socket.IO `message_from_socket`. Ported 1:1
including branch order. `<|§|>` is replaced with a space before matching;
non-empty commands are logged. Most branches return `{success:true}`;
`/volume`, `/spotify`, `/obs`, `/exec`, `/batch`, `/fetch`, `/usage`, soundboard, and
`/firstplan`-not-found return their own payloads.

`/bypass-windows-firewall` runs first (firewall fix) without consuming the
message. `/debug-send <json-ish>` only logs the parsed payload.
`/exit` terminates the process.

## Prefix map

| Prefix | Action |
| --- | --- |
| `/usage …` | Return `get_usage()` snapshot (the route serves this; branch kept 1:1) |
| `/stop_sound` | Stop soundboard playback |
| `/playsound …`, `/playlocalsound …` | Play a file (volume, ear-monitor, local-only params) |
| `/PCshutdown`, `/PCrestart`, `/PCsleep`, `/PChibernate` | Power actions (`shutdown`/`rundll32` on Windows, `systemctl` on Linux) |
| `/locksession` | Lock (`LockWorkStation` / `loginctl lock-session` → ScreenSaver bus) |
| `/screensaversettings` | Open saver settings |
| `/screensaver[ on/start \| hard/full/black \| off/false]` | Start saver / DPMS off / wake |
| `/key <name>` | Press a key (enigo) |
| `/restartexplorer` | Restart Explorer (Windows) / plasmashell (KDE only) |
| `/kill`, `/taskill`, `/taskkill`, `/forceclose <name>` | Close window by title, else `taskkill /f /im` / `pkill -x` |
| `/restart <exe>` | Kill + relaunch a program |
| `/clearclipboard` | Clear clipboard (`clip` / arboard) |
| `/write <text>`, `/writeandsend <text>` | Type text (optionally + Enter) |
| `/appvolume +\|-\|set …` | Per-app volume (COM `windows` crate) |
| `/soundcontrol mute` | Toggle sink mute (`pactl` on Linux) |
| `/mediacontrol playpause\|previous\|next` | Media keys (MPRIS on Linux) |
| `/speechrecognition` | Win+H hotkey |
| `/superAltF4` | Close focused window + kill fallbacks |
| `/firstplan <window>` | Focus window + Enter (error result if missing) |
| `/setmicrophone <name>` | Default input device |
| `/setoutputdevice <name>` | Default output device |
| `/copy [text]`, `/paste [text]`, `/cut`, `/clipboard` | Clipboard (arboard) + Ctrl+C/V/X, Win+V |
| `/volume …` | System volume (`audio::change_volume`; errors → `{success:false}`) |
| `/spotify …` | Spotify sub-dispatcher (see integrations) |
| `/obs …` | OBS sub-dispatcher (see integrations) |
| `/colorpicker …` | Screen color picker (see below) |
| `/openfolder`, `/opendir`, `/openfile`, `/start …` | Open path/URL (`system::handle_command`) |
| `/exec …` | Run Python code/file (`exec::python`) |
| `/batch …` | Run batch/shell code (`exec::batch`) |
| `/fetch …` | HTTP request to another app (`fetch::fetch`; see integrations) |
| `/<plugin-command> …` | rhai plugin commands (`plugin_commands()` registry) |

OS actions map: `subprocess.Popen(shell=True)` → `spawn_shell` (fire-and-forget,
`cmd /C` vs `sh -c`); `pyautogui`/`keyboard`/`pyperclip`/`win32gui` → `input`
helpers (enigo input, arboard clipboard, `windows` handles); Linux desktop
equivalents (systemd/logind/MPRIS/PipeWire) live in `desktop_linux`.

Two upstream quirks are fixed rather than ported: `/restartexplorer` and
`/kill` no longer pass an HWND int into `close()` (Python raised `TypeError`
and could close a random window) — they close by exact title.

## Usage tiles

`src/app/buttons/usage/`: `get_usage(example, asked_devices)` aggregates
CPU/RAM/disks/GPU (`gpu.rs` NVML, `gpu_amd.rs` amdgpu probe, `disks.rs`,
`asked_devices.rs`). GPU method comes from `settings.gpu_method`
(see startup). The frontend polls `POST /usage` on a timer and renders
tiles; `/api/boot` ships a `usage_example` snapshot for first paint.

## Color picker

`src/app/buttons/color_picker/`: `command_handler` parses `/colorpicker`
args (`get_arg`), captures the pixel under the cursor
(`get_mouse_pixel_color`), names it (`get_color_name` via `colors.json`),
and notifies (`notification`, clipboard copy).

Linux capture deliberately avoids the `screenshots` crate: it pulls in
dbus/vendored libdbus built without `HAVE_POLL`, so any D-Bus call with an
fd ≥ 1024 ends in a stack-smashing SIGABRT (compounded by `sysinfo`
fd pressure — hence `System::new()` over `System::new_all()`).
Linux uses `grim` (subprocess) + ashpd portal (pure Rust) + raw xcb,
with one fresh `zbus::Connection::session()` per portal capture (ashpd's
cached global connection cannot outlive the throwaway runtimes).
`screenshots` stays only on non-Linux targets, where its dbus dependency
never builds.

## Button catalog (`webdeck/commands.json`)

Nested `{category: {label: {command, args, style}}}` plus
`{TYPE:"multiple", commands:[…]}` entries. Categories include Webdeck,
Display (usage tiles), and the integration/system groups. The frontend
Add-button browser renders this catalog; `args` entries use the arg-schema
`TYPE` protocol (see frontend-editor).
