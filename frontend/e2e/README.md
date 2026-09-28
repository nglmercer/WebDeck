# Demo video (Playwright)

Records a scripted full-tour video of the WebDeck frontend at 1280x720:

```sh
cd frontend
npm run demo:video
```

Output: `frontend/demo/webdeck-demo-720p.mp4` (H.264 when `ffmpeg` is on
`PATH`, otherwise the native `webdeck-demo-720p.webm` is kept).

## Tour coverage (`e2e/demo-video.spec.ts`)

1. Boot: loading screen → grid, usage tiles fill in.
2. Folder navigation: index → spotify → index.
3. Config modal: Themes & backgrounds library tabs (backgrounds, themes).
4. Editor mode: rename a button via the Appearance tab, save in place (no reload).
5. Add a button: void slot → live search (`lock`) → System › Lock session → save.
6. Swap mode: pick two tiles, trade places, exit swap mode.
7. Save & exit, press a command button, usage tiles repopulate.

## Layout

- `demo-video.spec.ts` — thin orchestrator: cursor, mocks, event recorder,
  step calls in tour order, video publish.
- `demo/steps.ts` — one exported function per tour section; reorder or
  add steps here without touching harness code.
- `demo/mocks.ts` — fixture loading, route stubs, stateful grid replay.
- `demo/actions.ts` — `vclick`/`vfill`: glide the rendered cursor, then act.
- `demo/cursor.ts` — rendered pointer (arrow + click ripple) injection.
- `demo/events.ts` — recorder for the app lifecycle events plus
  `waitForAppEvent`: steps wait on real transitions (save finished,
  refresh rendered) instead of fixed sleeps.
- `fixtures/` — `boot.json`, `usage.json` backend snapshots.

## App events

The app emits lifecycle events on `window` as `webdeck:<name>`
(see `src/app/events.ts`): `boot:ready`, `app:refreshed`,
`usage:updated`, `editor:changed`, `save:completed`,
`server:disconnected`, `server:reconnected`. The tour records them via
`addInitScript` and waits with `waitForAppEvent(page, name, fromIndex)`.
DOM assertions stay as the final proof; events carry the causal wait.

## How it works

- Playwright starts `vite dev` and records the tour above.
- The Rust backend is mocked from `e2e/fixtures/` (`/api/boot`,
  `/get_config`, `/usage`, `/send-data`, save endpoints, `/create_folder`)
  and `/static/*` is served from the repo copy, so the shoot is
  deterministic and triggers no real button actions on the host.
- The mocks are stateful: `/save_buttons_only` and `/save_config` capture
  the posted grid and later `/api/boot` + `/get_config` calls replay it,
  so added/renamed/swapped buttons persist across the tour's in-place
  refreshes like a real server.
- Playwright's encoder does not capture the OS pointer, so the spec
  injects a rendered cursor (arrow + click ripple) via `addInitScript`,
  which survives page reloads (the `/reload` command still reloads). Clicks glide to their target
  (`vclick`/`vfill` helpers) so pointer motion reads on video.

## Identifier conventions

Demo selectors must survive refactors, so product markup carries stable
hooks. Rules for new UI:

- Prefer `data-testid="..."` for e2e/demo targets. Never style on it and
  never rename it without updating `e2e/`.
- Prefer semantic hooks over positional ones: `[data-message="/..."]`
  survives fixture reordering, `#button_e0X3` and `nth()` do not.
- Generated ids that start with a digit (`2X5_submit`) are invalid CSS —
  select them via `[id="..."]` or, better, a `data-testid`.
- Current contract (asserted by the tour): `deck-tile` + `data-message`
  on grid buttons, `add-slot` on void plus tiles, `add-leaf` +
  `dropdown-commandtag` (catalog key, e.g. `Lock session`) on catalog
  entries, `add-args-save` on the add-args submit, `lib-tab-themes` /
  `lib-tab-backgrounds` on the config library buttons.
- Hidden modals keep layout with opacity 0: assert the `display` flip
  (`toHaveCSS('display', 'block')`), not `:visible`, which is vacuous there.

## Live mode

To record against a real server instead (no mocks — button presses will
execute for real):

```sh
WEBDECK_DEMO_BASE_URL=http://<host>:<port> npm run demo:video
```
