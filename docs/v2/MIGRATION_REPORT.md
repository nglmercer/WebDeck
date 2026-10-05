# From-scratch v2 rewrite

This rewrite replaces the text-command runtime, JSON-value configuration model,
compatibility facades, global state and imperative frontend. Audited OS primitives
for screenshot capture, endpoint COM selection, QR rendering and updater rollback
were retained under the new owners; they contain no v1 application protocol.

| Removed surface | Canonical replacement |
| --- | --- |
| `/send-data`, text prefixes and delimiters | Typed `/api/v2/commands` requests |
| Root socket namespace and old command events | `/v2`, `command`, correlated `command_result` |
| `/api/boot` | `/api/v2/boot` with private action references |
| `/get_config` | `GET /api/v2/config`, Settings capability |
| `/save_config`, `/COMPLETE_save_config` | `PUT /api/v2/config`, mandatory revision |
| `/save_single_button`, `/save_buttons_only` | ID-based button mutations or a revision-aware config transaction |
| `/create_folder` | Revision-aware `/api/v2/folders` |
| `/usage` | `/api/v2/usage` or correlated v2 usage events |
| `/upload_file` | `/api/v2/assets`, opaque asset IDs |
| `/upload_filepath`, `/upload_folderpath` | Local-only `/api/v2/native/selection` |
| `/.config/...`, `**uploaded/...` | Protected asset IDs or explicit privileged external sources |
| Stringified arrays/booleans/numbers and old grid slots | Typed layout/settings and stable ordered IDs |
| Python/prefix plugin registry | Versioned manifests, typed arguments/results, JavaScript `invoke_action` |
| Python-named script command and shell argument parsing | Explicit inline/file JavaScript and shell sources |
| Implicit saves, folder queues and global dispatch | ConfigStore transactions and feature-owned drafts |

The source checkout and generated artifact contain one runtime. Rejection fixtures can mention old interfaces, but first-party production
clients cannot call them. Existing incompatible user configuration is rejected
without conversion or deletion.

The new editor handles folder/button creation, editing, ordering and deletion,
revision conflicts, theme/background/image uploads, typed command and plugin
forms, settings, backup drafts, explicit capability grants, device revocation,
fullscreen/reload/navigation and live system usage. Native dispatch consumes typed
arguments directly, including key lists, volume changes, integration actions,
HTTP headers/body/timeouts and script/file sources.

Validation results and limits are recorded in `evidence/rewrite/checks.json` and
`STATUS.md`. Tests use isolated user data and fake native effects; destructive
native actions were not invoked. The extracted portable package and real updater
rollback are validated separately from source/browser tests. This report makes no before/after performance claim.

Repository settings, release publication and artifact upload are outside this change.
