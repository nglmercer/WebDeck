# Configuration and backups

Only canonical schema version 2 is accepted. The document contains `settings`, `layout` and explicit `extensions`; folders and buttons use stable IDs and ordered arrays. The schema is [v2.schema.json](../contracts/v2.schema.json).

The default directory is `.config`, overridable by `--config-dir` or `WEBDECK_CONFIG_DIR`. It holds `config.json`, device sessions, integration tokens, plugins and `user_uploads`. Keep this private runtime data outside version control.

`ConfigStore` validates before saving, uses process writer locks and atomic replacement, and fingerprints disk bytes into revisions. External invalid edits block writes while preserving the last valid snapshot. Concurrent changes produce conflicts without discarding the editor draft.

Settings exports canonical backups and imports them into a draft; saving still requires the loaded revision. Older documents are rejected without automatic conversion, including documents with the old `front` layout. Archive them before starting with an empty directory. Do not change their version marker to bypass validation.

Translations remain `.lang` files under `webdeck/translations`. Themes, backgrounds and button images are uploaded assets referenced by IDs in the canonical layout.
