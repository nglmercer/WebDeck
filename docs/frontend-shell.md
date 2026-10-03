# Frontend

`frontend/src/main.ts` mounts the Svelte application. `api.ts` validates API contracts and handles paired identity; `App.svelte` owns navigation, deck state, settings and live usage. The app loads `/api/v2/boot`, the command catalog and translations, then connects to `/v2` Socket.IO.

Controller boot returns action references without integration credentials or private command sources. Remote controllers enter a token approved on the host. Tokens go in headers or socket authentication, never URLs.

Folders and buttons retain stable IDs. The deck renders uploaded images, backgrounds and themes through protected asset retrieval. Usage is read through `/api/v2/usage`. Disconnection clears pending observers and never replays native commands automatically. Styles live in `frontend/src/style.css`.
