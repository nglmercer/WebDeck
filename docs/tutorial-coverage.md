# Tutorial coverage

The audit starts from commit `7778278`. Driver.js 1.4.0 is already installed and loaded on demand by `Learn.svelte`.

At that commit, Learn offers one overview tour and a local practice deck. The overview has two real targets (deck and an optional folder); editing, connecting and replay use unanchored explanation cards. The three browser scenarios verify overview navigation/replay, viewer restrictions and isolated practice controls. They do not cover a tutorial for the editor or any settings workflow.

## In-app coverage

| Guide | Existing coverage | Required surfaces |
| --- | --- | --- |
| Overview | Partial Driver.js tour | Deck, folders, actual editor toolbar, device connections, help/replay |
| Controls and navigation | Static shortcuts text | Controls menu, navigation, keyboard shortcuts, viewer permissions |
| Editing and folders | One explanation card | Edit toolbar, add cell, folder actions, draft/save/Done |
| Button content and images | None | Label, preview, icon/image choice, file drop, URL/local imports, library |
| Button appearance | None | Color, dimensions, icon size, label visibility |
| Actions and automation | None | Action catalogue, categories/search, configuration, workflows, plugins |
| Keyboard shortcuts | None | Recording, manual selection, categories, modifiers, simultaneous keys, Apply |
| Audio fragments | None | File selection, waveform, start/end, preview, volume, output, Apply |
| Layout, language and themes | None | Dimensions, style, preview, language, backgrounds/themes, Save |
| OBS and Spotify | None | Configuration, hidden credentials, explicit connection checks and authorization |
| Device connection and access | One explanation card | Connect entry, QR/code/approval explanation, devices, access profiles and expiry |
| Advanced network and recovery | None | Listener/address/port, transport, diagnostics and recovery |
| Backups | None | Inspect/preview/apply/save, export and conflict recovery |
| Runtime and plugins | None | Runtime health, installed/disabled plugins and refresh |
| Phone pairing | None | Device name, remembered access, request/code approval and manual token |
| Practice | Local controls only | Driver.js coaching on the existing isolated practice controls |

Written installation, CLI, API, native OS and developer guides remain documentation: Driver.js cannot highlight a native installer, terminal or another computer. Their relevant browser entry points belong in the in-app guides above.

## Verification requirements

- Every guide must be discoverable, replayable and localized in English and Spanish.
- Steps must prepare their actual UI context before highlighting a visible target. Missing targets must produce a recoverable failure rather than an unrelated floating card.
- Viewer and pairing sessions must only offer guides they can actually open.
- Forward/back navigation, Escape/close, focus restoration, reduced motion and narrow screens must work, including steps inside dialogs.
- Walkthroughs must preserve persisted configuration and existing drafts. Save, commands, uploads, networking changes, authorization and device grants require deliberate user actions outside a walkthrough.
- Editor lessons use disposable examples; the practice lesson only changes its local demo state.
- Browser coverage must traverse every declared step, verify visible targets and layering, check side effects, and exercise cancellation/replay.

## Implemented coverage and evidence

The catalogue in `frontend/src/features/learn/guide-data.json` provides all 16 guides and 90 anchored steps in English and Spanish. Learn has search, category filters, replay and optional local completion tracking. Editor, settings and pairing headings offer contextual help. The catalogue, runner and Driver.js load on demand.

`GuideRunner` waits for each prepared target before advancing, supports previous/next, Escape and focus restoration, and adjusts native dialog layering while a tour is active. Its interaction guard blocks real configuration changes and commands. `GuideWorkspace` restores the previous navigation context, keeps real unapplied editor input mounted, and uses disposable buttons, folders, images, synthesized audio and pairing previews for lessons.

Validation for this change:

- `guides.spec.ts` and `learn.spec.ts`: 21 passing browser tests, including traversal of every declared step, visible targets and dialog layering, unchanged persisted configuration, no API writes, editor draft/focus restoration, pairing, viewer restrictions, completion/replay and a 375 px audio walkthrough with an accessibility check.
- `guides.test.ts`: independently enumerates the required guide topics, checks bilingual copy and validates role/step filtering. The frontend unit suite passes 100 tests across 17 files.
- Existing connection, keyboard shortcut, media editor and settings suites pass 18 browser regression tests.
- Component and TypeScript checks, unused-code analysis, translation validation, generated contracts, production build and source formatting pass.
- Bundle measurement is recorded in `v2/evidence/improvements/tutorial-bundle.json`: 468496 bytes JavaScript / 135232 bytes gzip across all chunks, including deferred code. The documented bundle ceilings account for the complete bilingual catalogue; UI timing budgets are unchanged.

The browser suite traverses all guide targets at desktop size; the dedicated narrow-screen scenario covers the audio guide and catalogue. English is exercised in browser tests; both English and Spanish resources are checked by the coverage contract.
