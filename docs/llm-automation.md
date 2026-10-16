# LLM automation with WebDeck

Use `webdeckctl --json` and parse its single stdout envelope. Diagnostics go to
stderr. Success is `{"ok":true,"data":...}`; failures retain the existing typed
error envelope and documented exit codes. A partial OBS apply is a failed outcome
with recovery data. Never infer success solely because some operations applied.

```sh
webdeckctl action describe obs --json
webdeckctl obs capabilities --json
webdeckctl obs collections --json
webdeckctl obs scenes --json
webdeckctl obs scene items Gameplay --json
webdeckctl obs inputs --json
webdeckctl obs hotkeys --json
webdeckctl obs project schema --json
webdeckctl obs project init ./stream-project --json
webdeckctl obs project validate ./stream-project --json
webdeckctl obs project diff ./stream-project --json
webdeckctl obs project apply ./stream-project --dry-run --json
```

Generate files against [the project schema](../contracts/obs-project.schema.json).
Use stable logical scene/item IDs and `inputId` for renameable inputs. Resolve
all include paths relative to `project.json`. Use a nested shared scene for
reusable overlays; do not copy its input definitions into every parent.
Start with empty scenes or explicitly existing inputs, since captures and source
settings depend on the host. Never invent input kinds, local paths or services.
Never put OBS credentials, stream keys or tokens in a project or prompt output.

Before live apply, inspect `data.plan.conflicts` and every non-noop operation.
Select a designated test collection; do not mutate an unknown active collection.
`--yes` does not authorize destruction. Explicit adoption and deletion flags must
reflect the user's actual authorization. Include `--revision` when applying deck
changes from a previously read revision. Handle revision conflicts by re-reading
and recomputing, never overwriting user changes.

After an authorized merge, call `project apply`; then diff again and expect only
no-op operations. On `partial_failure`, retain `.webdeck-state.json`, inspect the
journal and OBS state, and retry through diff. Never blindly replay a timed-out
mutation or promise rollback. See [OBS projects](obs-project.md) for limits,
managed pruning, button sync and recovery.

Named OBS hotkeys returned by `GetHotkeyList` use action `hotkey`, which now calls
`TriggerHotkeyByName`. Existing saved `hotkey` actions with raw `OBS_KEY_*` IDs
should change to `hotkey_sequence`; optional `key_modifiers` holds `shift`,
`control`, `alt`, and `command`. Optional `context_name` scopes named hotkeys.
Prefer dedicated `scene`, recording, streaming and mute actions when available.

For keyboard chords, inspect [the shared key catalog](../contracts/keys.json).
`{"type":"key","keys":["ctrl","shift","f12"]}` is a simultaneous chord.
Unknown/duplicate keys fail before injection. `fn`, menu and consumer keys are unavailable. Numpad digits and supported lock/system keys have distinct catalog IDs. Use dedicated media actions when appropriate.
Arbitrary text belongs in `write`. Browser shortcut capture is focused and local;
it does not intercept global host shortcuts. Desktop/portal authorization still
must be granted on the actual host.
