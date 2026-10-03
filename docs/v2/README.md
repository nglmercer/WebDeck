# WebDeck v2

The current application uses canonical typed v2 contracts, revision-aware storage, paired devices and a Svelte UI. Start with the [documentation index](../index.md) or [source quickstart](../../README.md).

[STATUS.md](STATUS.md) describes module ownership, execution policy, data protection and platform limitations. [MIGRATION_REPORT.md](MIGRATION_REPORT.md) maps removed interfaces to their replacements. [ACTION_INVENTORY.md](ACTION_INVENTORY.md) groups typed commands by capability. Current validation evidence is under `evidence/rewrite/`.

Existing incompatible configurations are rejected without runtime conversion. Use a canonical document or an empty data directory; archive older configurations before replacing them. Automatic updates remain disabled. Verified manual update and rollback mechanisms remain available.
