# Demo video (Playwright)

Records a scripted full-tour video of the WebDeck frontend at 1280x720:

```sh
cd frontend
npm run demo:video
```

Output: `frontend/demo/webdeck-demo-720p.mp4` (H.264 when `ffmpeg` is on
`PATH`, otherwise the native `webdeck-demo-720p.webm` is kept).

## How it works

- Playwright starts `vite dev` and records `e2e/demo-video.spec.ts`: boot,
  folder navigation, config modal, editor mode, button rename + save, and a
  command-button press.
- The Rust backend is fully mocked from `e2e/fixtures/` (`/api/boot`,
  `/get_config`, `/usage`, `/send-data`, save endpoints) and `/static/*` is
  served from the repo copy, so the shoot is deterministic and triggers no
  real button actions on the host.

## Live mode

To record against a real server instead (no mocks — button presses will
execute for real):

```sh
WEBDECK_DEMO_BASE_URL=http://<host>:<port> npm run demo:video
```
