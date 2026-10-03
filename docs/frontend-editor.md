# Editor

`editor.svelte.ts` owns feature drafts; `App.svelte` and `Fields.svelte` provide typed editing forms. Settings access requires its own capability. Editing supports folders, buttons, order, dimensions, typed commands, plugins, themes, backgrounds and images.

The editor validates the canonical configuration before `PUT /api/v2/config`. A save includes the revision originally loaded. Conflicts preserve unsaved work so it can be reconciled with the current document. Imported backups become drafts and follow the same save path.

Device approval and revocation require a local administrator. Remote controller boot exposes private action references rather than editable action definitions.
