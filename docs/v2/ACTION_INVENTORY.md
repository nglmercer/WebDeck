# Current action inventory (not a completed v2 contract)

Source: current `src/domain/command.rs` registry, `src/adapters/platform`,
`src/adapters/integrations`, `webdeck/commands.json` and frontend interaction code.
This inventory records preserved implementations and outstanding migration work.
The old strings below are **not** a supported final v2 contract.

| Current registry action | Old command prefixes | Capability | Migration ownership |
| --- | --- | --- | --- |
| `Debug` | `/debug-send` | `Read` | Native/integration adapter; typed replacement unfinished |
| `Exit` | `/exit` | `Power` | Native/integration adapter; typed replacement unfinished |
| `Usage` | `/usage` | `Read` | Native/integration adapter; typed replacement unfinished |
| `StopSound` | `/stop_sound` | `Audio` | Native/integration adapter; typed replacement unfinished |
| `PlaySound` | `/playsound `, `/playlocalsound ` | `Audio` | Native/integration adapter; typed replacement unfinished |
| `Shutdown` | `/PCshutdown` | `Power` | Native/integration adapter; typed replacement unfinished |
| `Reboot` | `/PCrestart` | `Power` | Native/integration adapter; typed replacement unfinished |
| `Sleep` | `/PCsleep` | `Power` | Native/integration adapter; typed replacement unfinished |
| `Hibernate` | `/PChibernate` | `Power` | Native/integration adapter; typed replacement unfinished |
| `Lock` | `/locksession` | `Power` | Native/integration adapter; typed replacement unfinished |
| `ScreensaverSettings` | `/screensaversettings` | `Power` | Native/integration adapter; typed replacement unfinished |
| `Screensaver` | `/screensaver` | `Power` | Native/integration adapter; typed replacement unfinished |
| `Key` | `/key` | `Input` | Native/integration adapter; typed replacement unfinished |
| `RestartDesktop` | `/restartexplorer` | `Window` | Native/integration adapter; typed replacement unfinished |
| `Kill` | `/kill`, `/taskill`, `/taskkill`, `/forceclose` | `Window` | Native/integration adapter; typed replacement unfinished |
| `Restart` | `/restart` | `Window` | Native/integration adapter; typed replacement unfinished |
| `ClearClipboard` | `/clearclipboard` | `Input` | Native/integration adapter; typed replacement unfinished |
| `Write` | `/write ` | `Input` | Native/integration adapter; typed replacement unfinished |
| `WriteAndSend` | `/writeandsend ` | `Input` | Native/integration adapter; typed replacement unfinished |
| `AppVolume` | `/appvolume +`, `/appvolume -`, `/appvolume set` | `Audio` | Native/integration adapter; typed replacement unfinished |
| `Mute` | `/soundcontrol mute` | `Audio` | Native/integration adapter; typed replacement unfinished |
| `PlayPause` | `/mediacontrol playpause` | `Audio` | Native/integration adapter; typed replacement unfinished |
| `Previous` | `/mediacontrol previous` | `Audio` | Native/integration adapter; typed replacement unfinished |
| `Next` | `/mediacontrol next` | `Audio` | Native/integration adapter; typed replacement unfinished |
| `SpeechRecognition` | `/speechrecognition` | `Input` | Native/integration adapter; typed replacement unfinished |
| `CloseFocused` | `/superAltF4` | `Window` | Native/integration adapter; typed replacement unfinished |
| `Foreground` | `/firstplan` | `Window` | Native/integration adapter; typed replacement unfinished |
| `Microphone` | `/setmicrophone` | `Audio` | Native/integration adapter; typed replacement unfinished |
| `Speakers` | `/setoutputdevice` | `Audio` | Native/integration adapter; typed replacement unfinished |
| `Copy` | `/copy` | `Input` | Native/integration adapter; typed replacement unfinished |
| `Paste` | `/paste` | `Input` | Native/integration adapter; typed replacement unfinished |
| `Cut` | `/cut` | `Input` | Native/integration adapter; typed replacement unfinished |
| `Clipboard` | `/clipboard` | `Input` | Native/integration adapter; typed replacement unfinished |
| `Volume` | `/volume` | `Audio` | Native/integration adapter; typed replacement unfinished |
| `Spotify` | `/spotify` | `Network` | Native/integration adapter; typed replacement unfinished |
| `Obs` | `/obs` | `Network` | Native/integration adapter; typed replacement unfinished |
| `ColorPicker` | `/colorpicker` | `Window` | Native/integration adapter; typed replacement unfinished |
| `Open` | `/openfolder`, `/opendir`, `/openfile`, `/start` | `Window` | Native/integration adapter; typed replacement unfinished |
| `Exec` | `/exec` | `Script` | Native/integration adapter; typed replacement unfinished |
| `Batch` | `/batch` | `Script` | Native/integration adapter; typed replacement unfinished |
| `Fetch` | `/fetch` | `Network` | Native/integration adapter; typed replacement unfinished |
| `BypassFirewall` | `/bypass-windows-firewall` | `Admin` | Native/integration adapter; typed replacement unfinished |

Plugin names are currently dynamic entries from `load_plugins.rs`, using the old
metadata/host API and prefix dispatcher. Unknown messages fail at the public
executor; direct nested compatibility dispatch remains a blocker. Native tests
use fakes/pure parsing except the explicitly isolated process/update fixtures.

Frontend-only actions are folder navigation (`/folder`), reload (`/reload`),
fullscreen (`/fullscreen`) and settings (`/open-config`). Usage tiles/polling use
`/usage`. These must become separate typed client actions or typed read requests.
Editor create/edit/delete/swap/undo and navigation use the current feature state,
button grid and remaining revision-required config handlers. Theme/background,
grid and settings flows now send actual arrays/numbers/booleans where migrated.
Upload/native picker and managed/external asset models remain to migrate.

First-party HTTP/socket wrappers, console, demo mocks, acceptance, portable smoke
and performance tooling use the v2 boot/command interfaces. Their remaining
configuration/usage/upload consumers are listed in [STATUS.md](STATUS.md).
The console still accepts legacy human command syntax; typed console actions are
unfinished. Python-named Rhai script modules and compatibility plugin discovery/
registration/examples remain unfinished. Existing action implementations were
preserved; no native actions were replaced with success stubs.
