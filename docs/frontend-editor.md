# Editor

`frontend/src/features/editor/editor.svelte.ts` owns draft mutations, persisted revision, dirty generation, save serialization, and deletion recovery. `button-draft.svelte.ts` stages a cloned button until Apply; opening or cancelling a dialog does not mutate neighboring cells. `ButtonEditorDialog.svelte` provides a shared tile preview, bounded appearance controls, typed action controls, and schema fallback for uncommon/plugin arguments. `ActionFields.svelte` groups searchable commands by capability and provides focused keyboard, text, and open-target controls.

Browser navigation warns about changes in the main draft or unapplied button dialog. Opening an unchanged existing button does not trigger a warning; Cancel deliberately discards staged changes. New button candidates also count as unapplied work.

Apply updates the draft. Save validates the canonical configuration and sends its originally loaded revision with `PUT /api/v2/config`. Concurrent callers share one pending save. Edits made during saving remain dirty after the submitted generation commits, and Done refuses to leave until those later changes are saved. A successful write stays successful even if display refresh fails. Conflicts and failures retain the draft; Download backup exports that draft before a deliberate reload.

Starting another save clears the previous global success notice. A later conflict therefore cannot display the old Saved banner as if the latest draft committed. Device approval links visible name/permission guidance to its disabled button through an accessible description.

Duplication assigns a new stable ID and a free cell while preserving action data and unknown extensions. Deletion keeps one recovery snapshot. Undo is offered only while no later draft mutation has invalidated that snapshot. Moving a button is committed on Apply; neighboring cells are not swapped during dialog editing.

Undo compares recovered content with the latest persisted snapshot. Undoing an unsaved deletion back to saved content clears the dirty flag. If the deletion has already committed, restoring it remains dirty; a save completing after undo also reconciles that flag against the newly persisted content without replacing the recovered draft.

Backups are validated and previewed before Apply backup to draft. Preview, invalid input, and Cancel restore do not mutate the draft. Replacing dirty data asks about that specific loss; restoring still requires a separate revision-aware save. File inspection is limited to 16 MB and ignores stale completions.

Appearance, Integrations, Devices, Backups, and advanced Connection have separate settings components under `features/settings/`. Numeric controls reject blank, nonfinite, fractional, and out-of-range values before mutation. Integration secrets are hidden until explicitly revealed. OBS connection checks save first, perform identification/version only, have a three-second budget, and release their client. Spotify reports whether authorization is stored without claiming live account verification; its explicit continuation link avoids opening a popup after an asynchronous save.

Configuration editing, integration checks, device approval/revocation, and native selection require an uncredentialed local administrator with Settings capability. A supplied device token retains its grant even from loopback. Controller boot exposes button references and required capabilities, never integration credentials or private command definitions. One-time tokens clear after copying or hiding; device grants show readable permissions and expiry.

Dirty state compares the JSON-shaped draft with a cloned persisted baseline on editor changes and save completion. Returning a value to its saved value clears dirty; object key ordering does not affect the comparison, while array ordering and unknown extension values do. Generation still advances on edits to preserve save-race ownership.

On touch pointers, Edit and Remove controls stack inside each cell and editing rows have a 220px minimum height so both controls remain reachable in narrow columns. This changes only the editing presentation; saved grid positions and spans remain intact.

Generic schema fields keep numeric, JSON and identifier-pattern errors beside their controls and expose those errors through accessible descriptions. Invalid input blocks Apply until corrected. Schema array fields show translated minimum/maximum item guidance and associate it with Remove/Add controls. Device rows expose Approved, Revoking and Revoked state; pairing describes where to obtain a token and retains it after a recoverable failure.
