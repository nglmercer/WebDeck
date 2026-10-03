# WebDeck code, design, and modularization plan

Prepared 2026-10-02 from the repository baseline. This roadmap records the original findings and proposed work; file sizes and defect descriptions below refer to that baseline. Implementation and verification have since progressed: see [improvement-progress.md](improvement-progress.md) for delivered changes, evidence, and remaining work. Visual proposals require browser validation before implementation is considered complete.

## Goals and constraints

Make the control deck easier to use, make editing safer and clearer, and let contributors change one feature without understanding the entire application. Preserve the minimal normal deck: the existing acceptance suite explicitly expects no header, sidebar, or footer in normal use.

Keep Rust, Svelte 5, the canonical v2 schema, stable IDs, existing configuration, uploaded themes, and desktop behavior. Use incremental changes with reviewable commits. A framework migration, new service layer, or Cargo workspace is not currently justified.

The following are mandatory regression constraints:

- Remote controllers receive button references, never integration credentials or action source.
- Authorization happens on the server for every invocation, including nested calls.
- Invalid supplied credentials cannot fall back to local administrator access.
- Accepted commands are owned by the server; disconnects and uncertain timeouts never trigger automatic replay.
- Revision conflicts preserve drafts; restores continue to enter the draft before saving.
- Stable IDs, array order, fixed cell positions, atomic storage, and asset confinement remain intact.
- Keyboard chords stay atomic without serializing unrelated actions. Shutdown drains accepted work.

## Repository findings

| Area | Evidence | Planned response |
| --- | --- | --- |
| Frontend composition | `frontend/src/App.svelte` has 1,165 lines covering routing, loading, assets, usage, actions, editing, settings, devices, and dialogs | Split by feature and give state explicit owners |
| Editor state | `editor.svelte.ts` has only revision, draft, dirty, and saved handling; mutations and saving live in App | Move editor operations and save lifecycle into a testable editor controller |
| Transport and contracts | `api.ts` combines schema interpretation, defaults, HTTP, credentials, socket state, and assets | Separate schema helpers from transport and lifecycle |
| Forms | `Fields.svelte` recursively renders schema names, with generic error feedback | Keep the generic fallback; add metadata and specialized controls for common actions |
| Styling | `style.css` has 381 lines, hard-coded colors, global element rules, and positional selectors | Establish tokens and move feature styles alongside components |
| Assets | App reloads and revokes all asset URLs, fetches assets sequentially, and does not deduplicate repeated icon IDs | Introduce a bounded, deduplicated asset lifecycle |
| Feedback | `getDevices()` turns errors into an empty list; many labels bypass translation | Distinguish failures from empty states and centralize user-facing strings |
| Backend composition | `server.rs` has 746 lines; `native.rs` has 869 despite existing helper modules | Extract cohesive routes and action families while keeping shared policy central |
| Existing verification | CI already includes Rust checks, frontend checks, Chromium acceptance, Windows tests, and portable rollback smoke checks | Extend the existing gates for identified risks; avoid duplicating infrastructure |
| Platform evidence | `docs/v2/STATUS.md` documents untested live desktop, audio, integration, and release behavior | Maintain an explicit manual platform verification matrix |

## Phase 0 — establish the baseline

Priority: P0. Estimated effort: 1–2 focused working days.

1. Run existing contract, v2-only, Rust, frontend, and acceptance gates. Record current failures separately from new regressions.
2. Capture normal deck, edit mode, button dialog, settings, pairing, empty deck, offline, and save-conflict views at 360, 768, and 1440 CSS-pixel widths. Include a touch viewport and keyboard-only walkthrough.
3. Record production bundle size, cold boot time, asset request count, usage polling behavior, and interaction responsiveness on a representative deck and a large synthetic deck. Set budgets from measured results rather than invented targets.
4. Add focused behavioral regression cases where coverage is missing: saving while editing, cancel after moving a button, overlapping boot/assets requests, and malformed folder hashes.

Exit: reproducible baseline, documented failures, screenshots, and a short list of confirmed defects. Existing recorded rewrite evidence is historical evidence, not a substitute for this baseline.

## Phase 1 — fix state and lifecycle risks

Priority: P0. Estimated effort: 3–5 days. These changes precede large component extractions.

### Editor transactions

- Give every edit operation one owner: create, update, delete, move, resize, folder rename/delete, restore, and save.
- Track draft generation as well as persisted revision. `persistOnce()` currently replaces the draft with the returned snapshot; explicitly preserve edits made while a save is pending. Either disable mutations during saving or reconcile the saved generation with later edits; choose one documented behavior and test it.
- Stage button placement changes in the dialog. `moveButton()` currently mutates another draft button before Apply; confirm cancel semantics and make cancellation leave the draft unchanged.
- Handle save success separately from subsequent boot/asset refresh failure. A committed save must not be presented as an unsuccessful write because refresh failed.
- Show explicit clean, dirty, saving, saved, conflict, and failed states. A conflict offers draft export and deliberate reload/discard; defer automatic merging.
- Protect unsaved work during reload, import, and navigation that discards state. Keep any confirmation specific to actual draft loss.

### Request and resource lifecycle

- Prevent old boot or asset requests from overwriting newer state with an abort signal or generation check.
- Deduplicate asset IDs, reuse URLs for unchanged assets, fetch independent assets with bounded concurrency, and revoke URLs on replacement/disposal. Avoid revoking the visible asset before its replacement is ready.
- Centralize timer, event listener, socket observer, and object URL cleanup. Preserve the current usage polling protections against overlapping requests and hidden-tab work.
- Distinguish device-list loading, empty, and failed states instead of converting every error into an empty list.
- Parse folder hashes defensively; malformed percent encoding must not break navigation.
- Verify the `edit` action: `invoke()` currently returns without toggling edit mode. Make its behavior agree with its advertised action and authorization.

Exit: targeted regressions pass, concurrent edits survive saves, cancelled dialog changes do not alter neighboring cells, and repeated loads do not leak resources or apply stale state.

## Phase 2 — modularize the frontend

Priority: P1. Estimated effort: 4–6 days.

Proposed structure; create files only as their responsibilities are extracted:

```text
frontend/src/
  App.svelte                       # feature composition and application lifecycle
  lib/
    contracts.ts                   # generated contracts, moved via generator config
    schema.ts                      # resolve, validate, defaults, schema types
    api/
      http.ts                      # credentials, request/response handling
      realtime.ts                  # correlation, disconnects, observer cleanup
      client.ts                    # typed endpoint functions
    assets.svelte.ts               # URL ownership and deduplicated loading
    navigation.svelte.ts           # hashes and history
    i18n.ts                        # translation lookup and formatting
  features/
    deck/
      DeckView.svelte
      DeckCell.svelte
      DeckControls.svelte
      deck.ts                      # grid placement and readings
      execution.svelte.ts          # running states and action dispatch
      usage.svelte.ts              # metric polling lifecycle
    editor/
      EditorToolbar.svelte
      ButtonEditorDialog.svelte
      ActionFields.svelte
      Fields.svelte                # recursive schema fallback
      editor.svelte.ts             # draft, mutations, revision/save lifecycle
      appearance.ts                # typed parsing and defaults
    settings/
      SettingsView.svelte
      AppearanceSettings.svelte
      IntegrationSettings.svelte
      DeviceSettings.svelte
      BackupSettings.svelte
    pairing/PairingView.svelte
  components/
    Feedback.svelte
    Modal.svelte
  styles/
    tokens.css
    base.css
```

Boundary rules:

- Use typed props/callbacks for feature components. Use a small application context only for truly shared services; avoid a single global store containing all feature state.
- Presentational deck cells do not fetch data or edit configuration. Editor mutation methods own draft changes; settings uses the same editor rather than a competing draft.
- HTTP and realtime expose compatible terminal results while retaining acceptance and unknown-outcome semantics.
- Schema utilities do not depend on UI or transport. UI metadata supplies labels/examples but never becomes a second command schema.
- Normalize appearance extensions through typed helpers; preserve unknown extension keys when writing.
- Keep App mainly composition, ideally under roughly 250 lines. Treat this as a review signal, not a reason to split cohesive code artificially.
- Update generator paths, imports, contract checks, and documentation together if generated files move.

Extraction order: schema/transport → assets/navigation/usage → editor operations → deck components → editor dialog → settings/pairing. Run the relevant checks after each extraction, before redesigning that area.

Exit: each feature has clear state ownership; App contains no recursive forms or settings sections; generated contracts still match the schema; current acceptance behavior passes.

## Phase 3 — improve visual design and workflows

Priority: P1. Estimated effort: 5–8 days, including browser review.

### Design foundations

- Define semantic tokens for canvas, surfaces, text, muted text, borders, accent, success, warning, error, spacing, typography, radii, elevation, and motion.
- Keep the existing dark direction initially. Use restrained accents, consistent alignment, readable secondary text, and a clear primary action per panel.
- Scope editor/settings styles to their components. Keep intentional theme override points documented so uploaded themes remain useful.
- Validate custom button colors for readable labels. Offer contrast feedback and a suitable foreground rather than assuming all button colors suit the same text.
- Respect reduced motion, visible keyboard focus, zoom, and touch target sizes. Target at least 44×44 CSS pixels for ordinary touch controls where layout permits.

### Normal deck

- Retain the full-area grid and configurable fixed cells; mobile responsiveness must not silently rearrange saved positions.
- Add local pressed/running/failed/permission feedback where useful, with accessible labels and non-color indicators. Avoid a success toast for every routine action when local feedback is sufficient.
- Distinguish unavailable, stale, and live readings. Keep metric polling out of views that do not need it.
- Preserve right-click, touch hold, Q, F1, and Ctrl+, access. Offer a dismissible first-use hint without permanent chrome; keep administrator controls absent for controllers.
- For very dense grids, define deliberate scrolling and minimum readable sizes. Defer virtualization until measurements show a real need.

### Editing

- Use a clearly identifiable editing toolbar with folder context, add button/folder, save status, Save, and Done.
- Group button editing into Content, Action, and Appearance with a live tile preview. Keep Apply-to-draft distinct from Save-to-host.
- Make common actions easy to select through searchable categories, human-readable names, examples, and relevant controls. Retain schema-generated fields for less common/plugin actions.
- Show field-level validation before submission; handle empty numeric input without storing NaN. Keep server validation authoritative.
- Add a clear duplicate workflow and safe deletion feedback. Consider undo after editor operations are centralized. Add drag-and-drop only later, with keyboard/touch equivalents and span collision rules.

### Settings and pairing

- Group settings into Appearance, Integrations, Devices, and Backups; use compact section navigation or disclosure on small screens.
- Move the HTTP/realtime selector into an advanced connection section; it does not belong under Appearance.
- Show integration connection states and actionable failures without exposing secrets. Provide explicit reveal controls where appropriate.
- Display device permissions in plain language and expiry in readable units. Keep one-time token disclosure, add copy feedback, and clear token UI after use.
- Explain backup restore as loading a draft. Show validation failures and the loaded file before applying changes.
- Make pairing explain where the token comes from and retain recoverable input on a failed attempt.

### Accessibility and language

- Give every modal a name, predictable Escape handling, focus containment, and focus restoration. Review current overlay dialogs as well as the native button dialog.
- Announce errors and status without repetitive screen-reader interruptions. Explain disabled actions through accessible descriptions.
- Extract hard-coded interface strings and friendly capability/action names into translation keys. Verify long translations and missing-key fallbacks.
- Review keyboard-only flows, 200% zoom, small screens, contrast, and reduced motion in the browser. Add automated accessibility checks for core screens, supplemented by manual review.

Exit: reviewed screenshots for each core state/viewport, no accidental command activation during editing/hold gestures, preserved fixed cells and themes, and usable keyboard/touch flows.

## Phase 4 — targeted Rust modularization

Priority: P2. Estimated effort: 4–7 days. Keep a single crate.

1. Split server code into `server/auth.rs`, `server/commands.rs`, `server/config.rs`, `server/assets.rs`, `server/devices.rs`, and `server/integrations.rs` as cohesive route groups justify them. Keep router construction in `server.rs` and the existing realtime module.
2. Share identity, capability checks, and button resolution policy between HTTP/realtime without broadening visibility or duplicating local-admin rules. Retain router-level tests, not only helper tests.
3. Move command-family implementations from `native.rs` into focused modules such as audio, clipboard, processes, and system actions. Keep the typed command match and policy checks easy to audit; preserve existing capture/input/scripts/integration modules.
4. Separate process/audio resource ownership from command handlers. Make shutdown responsibilities explicit and retain existing input locking and nested execution context.
5. Inspect blocking work in async route handlers; use bounded blocking execution where measurements or code show a need. Do not add a global lock or automatically retry native effects.
6. Audit network prefix parsing, runtime `unwrap`/`expect`, subprocess budgets, poisoned locks, and error context. Confirm malformed network prefixes cannot panic or widen access; add regression tests for confirmed cases.
7. Keep domain validation, storage transactions, session grants, and updater rollback cohesive. Split them only when a concrete responsibility or change pattern warrants it.

Exit: the same routes, policy, contract serialization, action semantics, shutdown behavior, and portable checks pass on the supported targets.

## Phase 5 — performance, maintenance, and platform confidence

Priority: P2. Estimated effort: 3–5 days plus access to live platforms/accounts.

- Measure gains from asset deduplication and parallel loading against Phase 0. Set a documented bundle/load regression budget after initial measurements.
- Profile large-deck placement before optimizing `gridCells()`. Test sparse cells, spans, collisions, resize, and overflow; preserve positions and extension data.
- Lazy-load editor/settings code only if the production build and loading measurements justify it.
- Add a non-mutating frontend formatting check to CI; retain Svelte, TypeScript, knip, generated-contract, Rust, acceptance, and portable checks already present.
- Replace acceptance timing sleeps with observable readiness where possible. Split long scenarios into isolated behaviors with reliable setup.
- Keep behavior-focused unit tests for editor operations, schema validation parity, transport outcomes, and asset disposal. Do not add snapshot tests for every extracted component.
- Record manual checks for Windows, Linux X11, and Wayland: tray/QR, permissions, keyboard/clipboard, media/audio, capture, child cleanup, and shutdown. Test OBS/Spotify with suitable accounts before claiming live integration support.
- Add diagnostics for request ID, action category, duration, and outcome. Redact tokens, secrets, headers, and script source. Treat operational logging separately from user-facing feedback.
- Update architecture, frontend/editor guides, status, and contributor instructions to match the final structure. Preserve the distinction between fake-effect acceptance and real desktop verification.

Exit: measurable performance results, CI enforcing agreed checks, and platform evidence clearly distinguishing passed, unavailable, and untested behavior.

## Delivery sequence and effort

| Milestone | Deliverable | Depends on | Estimate |
| --- | --- | --- | --- |
| M0 | Baseline and confirmed defect backlog | — | 1–2 days |
| M1 | Editor/request lifecycle fixes | M0 | 3–5 days |
| M2 | Frontend feature boundaries | M1 | 4–6 days |
| M3 | Tokens, deck/editor/settings improvements, accessibility | M2; screen work can proceed incrementally | 5–8 days |
| M4 | Targeted server/native extraction | M0; keep changes separate from frontend PRs | 4–7 days |
| M5 | Performance budgets, documentation, live verification | Relevant earlier milestones | 3–5 days plus platform access |

Total planning allowance: approximately 20–33 focused developer days for one contributor, excluding unavailable device/account verification. Refine estimates after M0. Prioritize M1–M3 for the strongest user-visible benefit; M4 can follow without delaying interface improvements.

Suggested review units: baseline → save/cancel fixes → resource lifecycle → schema/transport split → deck split → editor split → settings split → design foundations → workflow redesign → server route split → native families → measured performance/platform documentation. Each review unit should be independently understandable and keep the application runnable.

## Validation and completion

Use the repository's existing commands:

```sh
node tools/contracts/generate.mjs --check
node tools/validation/v2-only.mjs
cargo fmt --all -- --check
cargo test --locked --all-targets
cargo clippy --locked --all-targets --all-features -- -D warnings
npm run check:components --prefix frontend
npm run typecheck --prefix frontend
npm run knip --prefix frontend
npm test --prefix frontend
npm run build --prefix frontend
npm run test:acceptance --prefix frontend
```

Follow README prerequisites for native libraries, component-check dependencies, the built server, and Playwright. Use the existing CI portable packaging and rollback checks for relevant storage, startup, updater, or packaging changes. Run targeted checks during development and the complete applicable gate before merging.

The work is complete when core workflows pass, conflicts and disconnects retain their safety properties, frontend and backend ownership is documented, screenshots and accessibility review support the design decisions, and performance/platform claims have evidence. No automatic updates, replacement command protocol, microservices, or wholesale dependency upgrades are required by this plan.
