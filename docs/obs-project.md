# Declarative OBS projects

WebDeck manages OBS through its configured WebSocket integration. No system Node,
Bun, or Python is needed at runtime. Projects target scene collections, scenes,
inputs and scene-item instances; OBS profiles, stream keys and service settings
are outside this format. Never edit OBS's internal JSON while OBS is running.

## Start with an isolated test collection

```sh
webdeckctl obs configure --host 127.0.0.1 --port 4455 --password-stdin
webdeckctl obs check
webdeckctl obs capabilities --json
webdeckctl obs collections --json
webdeckctl obs project init ./my-obs-project
webdeckctl obs project schema --json
webdeckctl obs project validate ./my-obs-project --json
webdeckctl obs project diff ./my-obs-project --json
webdeckctl obs project apply ./my-obs-project --dry-run --json
webdeckctl obs project apply ./my-obs-project --yes --json
```

`init` requires a new directory and never overwrites files. The generated example
creates Starting, Gameplay, BRB, Ending and a shared, empty CommonOverlay. It has
no machine-specific capture inputs, unavailable URLs or missing asset files.
The collection name defaults to `WebDeck Test Stream`; change it deliberately.
A missing collection can be created. An existing different collection must be
selected explicitly in OBS before diff/apply: discovery never switches it.
Use only an isolated collection until you have inspected the plan.

`--yes` is the existing noninteractive compatibility flag: the CLI never prompts.
It does not authorize deletion. `diff` and `apply --dry-run` only read OBS and
WebDeck; they never write project state, asset files, deck configuration or OBS.
Offline `validate` needs neither a running WebDeck nor OBS. Online diff checks
installed input kinds and advertised OBS request capabilities before apply.

## Format and identity

The authoritative [JSON Schema](../contracts/obs-project.schema.json) describes
`project.json` and the included scene files. `schemaVersion: 1` is required.
Includes resolve beneath the project root. Traversal, absolute include paths,
symlink escapes, duplicate IDs/names, repeated instances of the same source in
one scene, broken scene references, cycles and excessive graphs are rejected.
Errors contain `file`, `pointer`, `code` and an actionable message.

Scenes have stable logical `id`, mutable `name`, and `items`. Item kinds:

- `scene_reference`: `sceneId` names another declared scene. This creates a
  nested reference, so changing its contents updates every referencing parent.
- `input`: `inputName`, `inputKind`, optional `inputId`, optional `settings`.
  Use a stable `inputId` when you intend to rename an input later. Repeated
  declarations share the underlying input; declarations must agree.
- `existing_input`: `inputName` references an already installed input and does
  not create it or change its settings.

Items have stable `id`, optional `enabled`, partial `transform`, and `index`.
Unspecified properties are preserved. Explicit null in input settings is a JSON
setting value, not a request to delete an OBS object. Input settings remain
specific to each OBS input kind. Source-kind availability is checked online.
Local-file source settings are rejected until a secure host asset-binding flow
is available; configure a local asset in OBS and reference that existing input.
Credentials and credential-bearing URLs must stay out of project files.

`.webdeck-state.json` stores safe UUID/name/item-ID bindings, an endpoint
fingerprint, and the latest apply journal. Keep it private to the project and
exclude it from portable templates. Bindings are scoped to project ID, WebDeck
URL, configured OBS endpoint and collection. UUIDs preserve scene/input identity
through renames. OBS versions without UUIDs use names, and cannot safely prune
those resources. Reusing the same name after a bound UUID disappears produces
a conflict rather than silently taking ownership.

Existing name matches require `--adopt-existing`. Inspect the collection and
plan first: adoption explicitly grants management of the declared resources.
Unrelated scenes, sources, scene items and buttons remain unmanaged.

## Plans and apply

Plans have stable operation IDs, collection, resources, operation kinds,
request payloads, before/after summaries, conflicts and warnings. Settings
snapshots in plan output are redacted. Resolve every conflict before apply.
The planner creates all scenes before attaching nested references. Requests
are serialized through a shared authenticated host connection, with unique
request IDs, bounded payloads/timeouts and no automatic mutation replay.
Each mutation checks that the target collection remains selected.

Apply records safe bindings after successful operations and verifies final
scene state. It then updates buttons using the existing revision-aware config
API. `--revision REV` checks the expected deck revision. A concurrent config
change does not get overwritten. Reapplying desired state sends no OBS
mutations and leaves an unchanged deck revision alone.

OBS changes are live and are **not transactional**. On failure, output contains
`state: partial_failure`, the journal and recovery instructions. WebDeck does
not claim rollback. Retain an OBS-supported backup before major changes.

## Managed deletion and buttons

Default `merge` never prunes. With `sync.mode: managed`, explicit pruning can be
previewed and applied with both flags:

```sh
webdeckctl obs project diff ./my-obs-project --prune-managed --allow-destructive --json
webdeckctl obs project apply ./my-obs-project --prune-managed --allow-destructive --json
webdeckctl obs ensure-buttons --folder obs-scenes --sync --dry-run --json
webdeckctl obs ensure-buttons --folder obs-scenes --sync
webdeckctl obs ensure-buttons --folder obs-scenes --sync --prune-managed --yes
```

OBS pruning requires live UUID ownership for scenes/inputs and recorded source
identity for scene items. References from preserved resources prevent pruning.
The ordinary `obs ensure-buttons` command remains add-only. Sync marks generated
buttons using `webdeck.obs` extension metadata, follows scene UUIDs, updates OBS
targets, preserves customized labels and appearance, reports stale buttons, and
only deletes stale managed buttons with explicit opt-in. Exact earlier generated
recipe IDs can be adopted; custom buttons are never claimed merely because their
actions match. Without scene UUIDs, renames cannot be reliably inferred.

The project `webdeck` declaration can enable button generation/synchronization.
It uses scene discovery, including nested scenes that OBS exposes as selectable
scenes. Existing folder navigation and unrelated buttons are preserved.

## Recovery and limits

After a partial result:

1. Inspect OBS and the journal; the failed request may have reached OBS.
2. Restore the intended active collection and endpoint.
3. Run `diff` again; UUID-bound successful resources are reused.
4. Resolve conflicts and rerun apply with the current deck revision.

Do not delete the state file as a shortcut. Missing state requires explicit,
reviewed adoption. Project applies hold a local lock and have a two-minute work
budget; large operations can safely continue in a later apply. Files are limited
to 1 MiB, project graphs to 128 scenes and 2048 items, source settings to 32 KiB,
and live discovery to 128 scenes/256 inputs/256 items per scene.

`replace` is explicitly unsupported: the API has no transactional collection
backup/restore, and this implementation does not fabricate one. Make a separate
collection through OBS and use merge. Asset upload, multicanvas selection, filters, profile management, event-driven UI
refresh and concurrent request multiplexing are not exposed. Nested scenes provide shared contents. Ambiguous discovered scene names fail closed.

All live project requests require local admin and network capabilities. Generic
plugins cannot use the trusted OBS host primitive or obtain its credentials.
Remote devices retain ordinary capability-gated OBS button actions but cannot
invoke project filesystem or destructive management endpoints.

## Propagating managed instance properties

An included shared scene can declare `instanceProperties` with `enabled`,
partial `transform`, and/or `index`. Preview and apply these defaults to its
managed parent references with:

```sh
webdeckctl obs scene sync --all --template ./my-obs-project/shared/common-overlay.json --dry-run --json
webdeckctl obs scene sync --all --template ./my-obs-project/shared/common-overlay.json --json
```

Only parent items declared in the containing project and already bound in local
state are touched. Explicit per-parent values win; transform overrides merge by
field. This operation changes instance properties, not the shared scene's
contents or unrelated collections. Ordinary project apply uses the per-item
properties; it does not implicitly propagate template defaults.

Protocol reference: the [official OBS WebSocket API](https://github.com/obsproject/obs-websocket/blob/master/docs/generated/protocol.md).
Implementation evidence and outstanding scope are recorded in the
[implementation report](keyboard-obs-implementation-report.md) and
[real-machine checklist](keyboard-obs-manual-tests.md).
