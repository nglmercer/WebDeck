# WebDeck v2 documentation

Build the frontend with `npm ci --prefix frontend` and `npm run build --prefix frontend`, then run `cargo run`. Open the printed loopback URL. See the [source quickstart](../README.md) for native dependencies and LAN pairing.

| Guide | Contents |
| --- | --- |
| [Architecture](architecture.md) | Current modules and ownership |
| [Startup](startup.md) | Data directory, server and tray |
| [Server](server.md) | HTTP and Socket.IO interfaces |
| [Commands](commands.md) | Typed actions and execution policy |
| [Integrations](integrations.md) | Native effects, OBS, Spotify and plugins |
| [Configuration](state-config.md) | Canonical documents, assets and backups |
| [Frontend](frontend-shell.md) | Deck, authentication and usage |
| [Editor](frontend-editor.md) | Drafts, mutations and conflicts |
| [Build and release](build-release.md) | Packages and verified updates |
| [Reference](reference.md) | Contracts and CLI |
| [Cleanup](refactor.md) | Removed artifacts and validation |

[Implementation status](v2/STATUS.md) records platform limitations. [Migration report](v2/MIGRATION_REPORT.md) maps removed interfaces to v2 replacements.
