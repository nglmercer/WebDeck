# Four-scene test project

This project intentionally has no hardware captures, external services or assets.
CommonOverlay starts empty; add supported inputs or existing-input references
there to share contents across all four parent scenes.

Use `webdeckctl obs project validate ./examples/obs-project --json` offline.
Inspect diff against an explicitly designated test OBS instance before applying.
See [the project guide](../../docs/obs-project.md). Generated `.webdeck-state*`
files contain local bindings and should not be copied between OBS hosts.
