# Frontend

`frontend/src/main.ts` mounts the Svelte application. `api.ts` validates API contracts and handles paired identity; `App.svelte` owns navigation, deck state, settings and live usage. The app loads `/api/v2/boot`, the command catalog and translations, then connects to `/v2` Socket.IO.

Controller boot returns action references without integration credentials or private command sources. Remote controllers enter a token approved on the host. Tokens go in headers or socket authentication, never URLs.

Folders and buttons retain stable IDs. The deck renders uploaded images, backgrounds and themes through protected asset retrieval. Usage is read through `/api/v2/usage`. Disconnection clears pending observers and never replays native commands automatically. Styles live in `frontend/src/style.css`.


The normal screen contains only the editable button grid. There is no fixed header, sidebar or usage page. CPU, per-core CPU, memory, GPU, GPU memory, disk and clock readings are metric buttons; visible metrics share polling and expose a configurable update interval. Missing devices display “Unavailable”. Folder buttons navigate to a folder page. Newly created folders add a link in their parent and a Back button in the child.

Settings, reload, fullscreen and back shortcuts are normal removable buttons. Editing is toggled only with Q; F1 displays shortcut help. Right-click or hold the deck to reveal controls, `Q` toggles editing, and `Ctrl+,` opens settings. These work even with an empty deck. `Alt+Left` returns to the root, and browser back/forward restores folder navigation. All button shortcuts can be deleted. Every grid cell is rendered; empty cells show only an add icon while editing, and occupied cells have separate edit and remove controls. Removing a button preserves neighboring positions. Grid rows no longer cap the number of buttons; overflow creates additional rows. Appearance includes dimensions, spacing, corners, icons, labels and individual button spans, with uploaded CSS themes for further styling.
