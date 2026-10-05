# V2 cleanup

The current runtime has one typed protocol and one Svelte application. Superseded runtime trees, text-command dispatch, imperative frontend styles/images, showcase assets, the unused color database, historical implementation plans and superseded validation reports have been removed. Current application icons, translations, metadata and rewrite validation evidence remain.

Documentation now describes current modules and interfaces. `tools/validation/v2-only.mjs` rejects removed production surfaces. [The migration report](v2/MIGRATION_REPORT.md) records replacement interfaces; [status](v2/STATUS.md) records actual validation limits. User data recovery is separate from runtime schema handling.
