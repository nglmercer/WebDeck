# Frontend — Editor

Button editing: `EditModal` (single slot) + Add-button browser
(`addbutton/`), with `ArgsBlock` rendering per-command arguments from the
catalog schema.

## Files

- `views/EditModal.svelte` / `editmodal.ts` (+ `editmodal.test.ts`,
  `modalstyle.ts`) — edit one button: name, message, image/size, colors,
  per-command args; save → `POST /save_single_button`
  `{location_Folder, location_Id, content}`.
- `views/addbutton/` — `browser.ts` + `AddBrowser.svelte`: searchable catalog
  browser over `/api/boot commands` (categories → labels → command), with
  icon wells per row (`.wd2-rowicon`, dark `currentColor` for visibility).
- `views/ArgsBlock.svelte` (+ `ArgsBlock.test.ts`) — renders the selected
  command's `args` array as field widgets.
- `views/argschema.ts` (+ `argschema.test.ts`) — arg-schema `TYPE` parsing
  and normalization.
- `views/args.ts` (+ `args.test.ts`) — arg assembly into message fragments.
- `views/argvalues.ts` (+ `argvalues.test.ts`) — value coercion/defaults.
- `views/getcommand.test.ts` — `buildCommand` protocol coverage.
- `components/` — field widgets: `TextField`, `NumberField`, `ColorField`,
  `FileField` (upload → `**uploaded/` URI), `SelectField`, `SwitchField`,
  `KeyFieldView`, `SearchDropdownView`, `Preview`, `EditorStyle`,
  `Collapse`, icons (`button-icons.ts`, `icons.ts`).

## Arg schema (`TYPE` protocol)

Each catalog entry's `args` is an array of `{TYPE, value?, …}` descriptors.
`TYPE` is a space-separated kind + modifier, e.g.
`input usage-title-text`, `input webdeck_foldername`, `text`, `select …`,
`color`, `file`, `number`, `switch`, `key`, `search-dropdown`. `argschema`
normalizes them; `ArgsBlock` maps each to a field widget; `args`/`argvalues`
assemble the fragments; `buildCommand` joins base command + fragments into
the final `message` string (fragments separated per protocol, `<|§|>` where
the backend expects splittable args, e.g. plugin commands).

`{TYPE:"multiple", commands:[…]}` catalog entries render as stacked
sub-commands sharing one tile (e.g. combined usage tiles).

## Save flows

| Action | Endpoint | Body |
| --- | --- | --- |
| Edit one button | `POST /save_single_button` | `{location_Folder, location_Id, content}` |
| Add (via full editor) | `POST /save_buttons_only` | `{front:{buttons}}` |
| Settings change | `POST /save_config` | full config (merged server-side) |
| Import/replace all | `POST /COMPLETE_save_config` | full config (grid preserved, then resized) |
| New folder | `POST /create_folder` | `{name, parent_folder}` (queued, flushed on next save/get) |

Folder addressing in `save_single_button` is positional (index into
`front.buttons` key order) — the editor and backend must agree on order,
hence `preserve_order` in `serde_json`.

## Uploads in the editor

`FileField` uploads via `POST /upload_file` (multipart) into
`.config/user_uploads/`; the stored message/background references it as
`**uploaded/<file>`. `POST /upload_folderpath` and `/upload_filepath`
open native pickers (rfd/xdg-portal) and return paths for `/openfolder`-style
commands. Served back via `GET /.config/<dir>/<file>`.
