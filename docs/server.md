# Server

`src/server.rs` owns the axum router. All application API routes use `/api/v2`.

| Routes | Purpose |
| --- | --- |
| `boot` | Controller layout with private action references |
| `config`, `settings/boot`, `settings` | Privileged configuration reads and revision-aware writes |
| `folders`, `folders/{id}`, nested `buttons` | Layout mutations |
| `commands` | GET catalog, POST typed command request |
| `usage`, `audio/devices`, `translations` | Read models |
| `devices`, `devices/{id}` | Local approval and revocation |
| `assets`, `assets/{id}` | Upload and protected asset retrieval |
| `native/selection` | Local file selection |
| `spotify/connect`, `spotify/callback` | Integration authorization |

`/` serves `frontend/dist/index.html`; `/assets` serves the Vite bundle and `/static` serves application icons. Build the frontend before starting. Socket.IO uses `/v2` and correlated `command` / `command_result` events.

The guard validates identity, literal Host/Origin and capabilities. Remote devices must be paired; invalid supplied credentials cannot fall back to loopback privileges. Configuration writes require the current revision. See [contracts](../contracts/v2.schema.json) for request and response shapes.
