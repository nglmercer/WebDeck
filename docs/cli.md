# CLI

`webdeckctl` is the administrative command line client for WebDeck. It talks to
a running server over `/api/v2` and never edits files that the server owns
directly: every configuration change goes through revision-aware endpoints.

Business logic lives in the reusable [`src/admin`](../src/admin) module so the
CLI, HTTP tests and a future MCP server share one implementation.
[`src/bin/webdeckctl.rs`](../src/bin/webdeckctl.rs) only parses arguments and
renders results.

## Build and run

```bash
cargo build --bin webdeckctl
cargo run --bin webdeckctl -- status
./target/debug/webdeckctl --help
```

## Connection resolution

Options are resolved in this order; the first source that provides a value wins.

| Setting | Order |
| --- | --- |
| Server URL | `--url`, `WEBDECK_URL`, `~/.config/webdeck/config.toml`, `http://127.0.0.1:5000` |
| Bearer token | `--token`, `WEBDECK_ADMIN_TOKEN`, `WEBDECK_DEVICE_TOKEN`, connection file |
| Config directory | `--config-dir`, `WEBDECK_CONFIG_DIR`, connection file, `.config` |

The connection file is TOML:

```toml
url = "http://127.0.0.1:5000"
token = "device-token"
config_dir = "/path/to/server/data"
```

A request without a token from a loopback peer is a local administrator and
receives every capability. Supplying any token selects device identity rules, so
leave `--token` unset for local administration. `--config-dir` must point at the
same directory the server was started with; it is only used to locate
`<config-dir>/plugins` for plugin package operations.

## Output

Human output is the default. `--json` prints exactly one JSON envelope on
stdout and nothing else:

```json
{"ok":true,"data":{"revision":12}}
{"ok":false,"error":{"code":"revision_conflict","message":"Configuration changed","details":{"expected":11,"actual":12}}}
```

Logs, progress and error lines go to stderr in both modes. `-v`, `-vv` and
`-vvv` add request tracing on stderr. `--yes` is accepted for scripts; webdeckctl
is always non-interactive and never prompts.

## Exit codes

| Code | Meaning |
| --- | --- |
| 0 | Success |
| 1 | Generic failure (including failed health checks) |
| 2 | Invalid arguments or input |
| 3 | Cannot reach the server |
| 4 | Authentication or capability denied |
| 5 | Validation failed |
| 6 | Revision conflict or resource already exists |
| 7 | Resource not found |
| 8 | Integration failure (OBS) |
| 9 | Plugin failure |

The JSON error `code` values are `generic_error`, `invalid_arguments`,
`connection_failed`, `authentication_failed`, `validation_failed`,
`revision_conflict`, `not_found`, `integration_failed` and `plugin_failed`.

## Status, health and discovery

```bash
webdeckctl status
webdeckctl doctor
webdeckctl version
webdeckctl capabilities
webdeckctl schema            # full v2 JSON schema
webdeckctl schema --name Button
```

`status` summarizes version, revision, runtime health, folder/button counts,
plugin counts and integration states. `doctor` runs checks
(`server.reachable`, `admin.access`, `schema.version`, `config.valid`,
`runtime.healthy`, `plugins.consistent`, `integrations.*`); warnings do not
fail, any `fail` exits 1. `version` reports the server version next to the CLI
version.

## Configuration

```bash
webdeckctl config get                     # secrets redacted
webdeckctl config get --reveal            # includes secret values
webdeckctl config validate --file deck.json
webdeckctl config diff --file deck.json
webdeckctl config apply --file deck.json
webdeckctl config apply --file deck.json --dry-run
webdeckctl config apply --file deck.json --revision 12
webdeckctl config export --file backup.json
```

`config validate` with no `--file` validates the server configuration; use
`--file -` to read stdin. `config export` writes the complete configuration
including secrets with `0600` permissions, so `export` and `apply` round-trip.
`plan` and the top-level `apply` are aliases of `config diff` and
`config apply`.

A change list describes each difference:

```json
{"operation":"update_settings"}
{"operation":"create_folder","id":"ops"}
{"operation":"update_button","id":"play","folder":"media-controls"}
```

Change operations are `update_settings`, `update_layout`, `create_folder`,
`update_folder`, `delete_folder`, `create_button`, `update_button`,
`delete_button` and `reorder_buttons`.

## Folders and buttons

```bash
webdeckctl folder list
webdeckctl folder create media --label "Media"
webdeckctl folder update media --label "Media player"
webdeckctl folder ensure media --label "Media"
webdeckctl folder delete media
webdeckctl button list --folder media
webdeckctl button create --folder media --file button.json
webdeckctl button update --folder media --file patch.json
webdeckctl button ensure play --folder media --file button.json
webdeckctl button delete play --folder media
```

Every mutation accepts `--revision` and `--dry-run`. Folder and button inputs
are partial JSON objects: `button update` and `button ensure` merge the provided
fields over the stored button and preserve everything you omit. `folder ensure`
manages existence and label only; manage buttons with `button ensure`.
Creating a folder or button that already exists exits 6. Missing folders and
buttons exit 7.

Reports share one shape:

```json
{"operation":"button_ensure","revision":13,"previous_revision":12,"applied":true,"dry_run":false,"id":"play","folder":"media"}
```

## Actions

```bash
webdeckctl action list
webdeckctl action describe obs
webdeckctl action run debug --args '{"data":{"text":"hi"}}'
webdeckctl action run obs --args '{"action":"get_scenes","target":""}'
webdeckctl action run --file command.json
webdeckctl action run open --arg url=https://example.com --dry-run
```

`action run` builds a typed command from `type` plus arguments, validates it
against the contract, then posts it once. A transport failure after the request
is sent reports an unknown outcome and never retries.

## Plugins

```bash
webdeckctl plugin list
webdeckctl plugin inspect echo
webdeckctl plugin validate ./dist/echo
webdeckctl plugin disable echo
webdeckctl plugin enable echo
webdeckctl plugin reload
webdeckctl plugin init myplugin --dir ./myplugin --version 2.0.0
webdeckctl plugin install ./myplugin
webdeckctl plugin update ./myplugin
webdeckctl plugin uninstall myplugin
```

Package rules:

- The package directory must be named exactly like `webdeck.json`'s `id`, and
  the entry digest must match the entry file.
- Packages may not contain symbolic links; at most 256 files and 64 MiB.
- `install` and `update` stage a copy outside `plugins/` (`.webdeck-staging`),
  swap it in, and reload the runtime. `update` and `uninstall` keep a rollback
  copy (`.webdeck-rollback`); if the reload fails, the previous package is
  restored and the command exits 9.
- `install` of an installed plugin exits 9 (`use plugin update`); `update` or
  `uninstall` of a missing plugin exits 7.
- `enable`/`disable` are runtime scoped and reset when the runtime reloads or
  the server restarts. `builtin.*` plugins cannot be disabled or uninstalled.

## Catalog automation

```bash
webdeckctl integration list
webdeckctl integration status
webdeckctl integration check INTEGRATION_ID
webdeckctl button recipes
webdeckctl button generate --recipe RECIPE_ID --folder FOLDER_ID --dry-run
webdeckctl button generate --recipe RECIPE_ID --folder FOLDER_ID --revision REVISION
```

Plugins and built-ins publish the same metadata. Qualified plugin action IDs can
be run directly: `webdeckctl action run devices.select --args '{"device":"a"}'`.
Button generation is atomic, preserves existing buttons, and is idempotent.
See [catalog-driven automation](automation.md) for metadata and a working plugin.
Human folder/button lists display details; `--json` retains structured output.

## OBS

```bash
webdeckctl obs status
webdeckctl obs check
webdeckctl obs configure --host 127.0.0.1 --port 4455 --password-stdin
webdeckctl obs scenes
webdeckctl obs ensure-buttons --folder obs-scenes
webdeckctl obs current-scene
webdeckctl obs inputs
webdeckctl obs hotkeys
webdeckctl obs hotkeys --target VolumeUp
webdeckctl obs stream status|start|stop|toggle
webdeckctl obs recording status|start|stop|toggle|pause|resume
webdeckctl obs virtual-camera status|start|stop|toggle
webdeckctl obs actions
```

`obs configure` patches host, port and password on the stored settings; the
password is only read from stdin and is never printed or logged. `obs check`
exits 8 when the connection fails. OBS query and control actions run through
the command endpoint, so OBS failures exit 8 as well.

## Security notes

- `config get` replaces a non-empty `settings.obs.password` with
  `"password_configured"` and reduces `settings.spotify.client_secret` to a
  boolean unless `--reveal` is given.
- `config export` is the only command that writes secrets, and the file is
  created with `0600`.
- Secrets never appear in `--json` envelopes, error details or `-v` logs.

## Tests

`cargo test --test admin` runs the CLI suite against a real axum server on a
random loopback port, including subprocess checks for JSON purity and exit
codes. Against a live server:

```bash
WEBDECK_URL=http://127.0.0.1:5000 cargo test --features integration-tests --test admin
```

### Image assets

`webdeckctl asset list` prints saved image references. Use `asset import --path
/absolute/path/icon.png` to copy a server-local image, or `asset import --image-url
https://example.com/icon.svg` to download one. Add `--live` to retain the URL for
`asset refresh IMAGE_ID.svg`. These operations require a local administrator and
do not modify button configuration. Set the returned `asset:<id>` as the button
icon. See [image imports](automation.md#importing-images) for API and limits.
