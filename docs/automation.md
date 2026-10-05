# Catalog-driven automation

The shared engine consumes data-only declarations. Built-ins declare them in
[`contracts/automation.json`](../contracts/automation.json); plugin packages may
add an optional `automation` object to `webdeck.json`. Existing packages remain
valid. Definitions appear in `/api/v2/commands` alongside action labels,
argument schemas and optional result schemas. The schema remains the source of
truth: edit `contracts/v2.schema.json`, then run `node tools/contracts/generate.mjs`.

## Using the CLI

```sh
webdeckctl action list
webdeckctl action describe devices.select
webdeckctl action run devices.select --args '{"device":"a"}'
webdeckctl integration list
webdeckctl integration status
webdeckctl integration check devices.connection
webdeckctl button recipes
webdeckctl button generate --recipe devices.select --folder devices --dry-run
webdeckctl button generate --recipe devices.select --folder devices --revision REVISION
```

The [devices example](../examples/plugins/devices) demonstrates discovery,
selection and a local health probe. Install it with
`webdeckctl plugin install examples/plugins/devices`. Its devices are sample data;
selection stores the chosen ID in the plugin's isolated storage.

For OBS, use the same engine:

```sh
webdeckctl integration check obs
webdeckctl button generate --recipe obs-scenes --folder obs-scenes
```

`obs check` and `obs ensure-buttons` remain convenience aliases. They do not
implement separate cache or provisioning logic. Integration-specific protocol
and credential handling stay in their existing adapters.

## Health declarations

An integration declares a unique `id`, a `label`, `required_settings`, and
`configuration`. Settings references are JSON pointers relative to the settings
object. Required values must exist and be non-null; strings must be non-empty.
The configuration pointers identify values that affect connectivity, including
credentials. They are fingerprinted in memory, never returned by the status API.

Optional `authorization_asset` names a confined file in the config directory.
Absence yields `authorization_required`; presence without a probe yields
`authorization_saved`. A saved token alone does not prove a live connection.

An optional `probe` declares a **read-only** command. Plugin authors must ensure
that the action has no control side effects. Checks are explicit; listing status
and running doctor do not execute probes. Caller capability checks, nested plugin
grants, deadlines and executor admission still apply.

Optional `success` selects a result value with a JSON pointer. With `equals`,
that value must equal the declared JSON value. Without `equals`, it must exist,
be non-null and, for a string, non-empty. Without a predicate, successful
execution indicates connectivity. Raw probe results and errors are not retained.

`/api/v2/integrations/status` returns an `integrations` map containing each
state and the actual last-check time in Unix seconds. Zero means no check has
been retained. A browser reload preserves the result. Relevant settings changes,
metadata changes and runtime reload or enable/disable operations invalidate it;
a server restart starts with an empty cache. The old `obs`, `spotify`, and
`checked_at` fields remain compatibility projections.

## Button recipes

Each recipe declares:

- `discovery`: the command that lists selectable items.
- `items_pointer`: a JSON pointer selecting the result array.
- `identity_pointer` and `label_pointer`: pointers relative to each item.
- `command`: a valid command template with initial values for its arguments.
- `bindings`: destination command pointers mapped to source item pointers.
- `folder_label`: the label used when creating the destination folder.

Bindings replace existing values; they cannot replace command or plugin identity
fields. All discovered items and bound arguments are validated before writing.
Missing fields, invalid types and duplicate identities fail the whole plan.

Provisioning creates a missing folder and missing buttons in one atomic,
revision-checked configuration write. Existing buttons with the same action are
preserved, including labels, colors, icons, IDs and extensions. Other buttons and
folders are preserved. Generated IDs derive from recipe, folder and item identity;
ID collisions receive a deterministic suffix. Repeating an unchanged recipe is
a no-op and does not bump the revision. Dry-run performs read-only discovery and
validation, but writes nothing. Removed discovery items are not deleted from the
deck automatically.

Every generated button wraps its command as
`{"type":"command","command":{...}}`. The same constructor works for built-in
and plugin commands.

## Result presentation

Presentations declare a command `selector`, a human label, and optional argument
labels mapped to command JSON pointers. The most specific matching selector is
used. Optional `result_pointer` selects the payload to render; plugin execution
uses an envelope, so the example selects `/value`. JSON output always preserves
the complete original result.

Optional `result_view` selects an item array and defines column labels mapped to
item pointers. Missing fields fall back to JSON display. Empty objects or null
payloads display a completion message rather than `{}`. Actions without metadata
still work with catalog labels and generic result display. Metadata never
executes templates, scripts, shell commands or expressions.

Plugin metadata IDs must start with `plugin_id.`. Its discovery, control and
health commands must reference that package's declared actions and version.
Presentations must target that plugin. Disabled packages are omitted from the
automation catalog; they cannot override built-in declarations.

Plugin status distinguishes available packages, enabled packages and active
sessions. Sessions load on first invocation, so zero active sessions is compatible
with a healthy runtime.

The browser consumes a generated schema graph that shares repeated objects. A
regression compares it with the full canonical definitions, preserving all
validation and editor behavior. The production bundle measures 229577 raw bytes
and 71309 gzip bytes; the raw limit remains 235000 and the gzip limit is 72000
(the previous measured baseline was 232606 raw / 70483 gzip). This accounts for
the additional automation contracts while reducing raw schema duplication.

## Icons

The renderer and searchable button picker share
`frontend/src/lib/icon-registry.json`. Register built-in SVG paths, style metadata
and legacy aliases there; icons are independent of command types. The picker
also reuses saved PNG, JPEG, WebP, GIF and SVG uploads from the real config
directory. Its image catalog is administrator-only, lists up to 128 images, and
displays 24 previews per page. Selecting an icon stages a button edit until Apply
and Save; uploading stores the image so it remains available after reload.

The picker adds approximately 3 KB raw / 1 KB gzip to the automation bundle; the
raw bundle ceiling stays 235000 and the gzip ceiling is 73000. Measurements are
recorded in `tools/validation/performance-budgets.json`.

## Importing images

In **Edit button → Content**, import a public HTTP(S) image URL, enter an absolute
file path on the WebDeck server, or choose a file with the native picker. Imports
copy the image into `user_uploads` and return `asset:<id>`. Changing or removing
the original file does not affect the saved icon. Imports persist immediately;
assigning the icon to a button still requires Apply and Save.

Enable **Keep URL for manual refresh** to retain a URL alongside its cached image.
**Refresh live image** replaces that cached image atomically and preserves its ID,
so every button referencing it sees the update. The current browser refreshes its
image caches; other open devices need a reload. Refresh is a shared asset change,
not a staged button edit, so cancelling the editor does not undo it. Failed
refreshes leave the last good image intact. Reads only serve the saved image and
never fetch a URL. There is no automatic refresh timer or arbitrary URL proxy.

```sh
webdeckctl asset import --path /absolute/path/icon.png
webdeckctl asset import --image-url https://example.com/icon.svg
webdeckctl asset import --image-url https://example.com/icon.svg --live
webdeckctl asset list
webdeckctl asset refresh IMAGE_ID.svg
```

The global CLI `--url` still selects the WebDeck server; `--image-url` selects the
remote image. Local paths refer to files on that server, not on a remote client.

API: `POST /api/v2/assets/import` accepts `{"type":"local","path":"..."}` or
`{"type":"url","url":"https://...","live":true}` and returns `FileSource`.
`POST /api/v2/assets/{id}/refresh` refreshes a retained URL. The asset list includes
`live_images` IDs. Imports, refresh, native selection and listing require a local
administrator. Existing authenticated asset reads and uploads keep their current
capabilities.

Imports accept PNG, JPEG, WebP, GIF and self-contained SVG graphics. Raster images
are decoded with limits of 8192 pixels per dimension and 64 MiB allocation; input
files/downloads are limited to 16 MiB. SVG imports reject scripts, external
references, styles, entity declarations and unsupported elements. URL downloads
validate and pin public destination addresses at each redirect, disable system
proxies, enforce request timeouts and reject HTTPS downgrade redirects. Saved live
URLs are private metadata in `image_sources`; browsers receive only asset IDs.

The import controls and contracts add 3876 raw / 1195 gzip bytes to the icon build.
The production build measures 236377 raw / 73471 gzip bytes; current bundle limits
are 240000 / 75000. Existing UI timing limits are unchanged.
