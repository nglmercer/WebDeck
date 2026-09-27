import { defineConfig } from '@playwright/test';

// Demo-video recording setup. Default mode records against `vite dev`
// with the Rust backend fully mocked (see e2e/demo-video.spec.ts), so the
// shoot is deterministic and triggers no real button actions.
// Live mode: WEBDECK_DEMO_BASE_URL=http://<host>:<port> npm run demo:video
// records against a real server with no mocks.
const liveBaseURL = process.env.WEBDECK_DEMO_BASE_URL;

export default defineConfig({
  testDir: './e2e',
  testMatch: 'demo-video.spec.ts',
  fullyParallel: false,
  workers: 1,
  reporter: 'list',
  outputDir: './e2e/test-results',
  use: {
    baseURL: liveBaseURL ?? 'http://127.0.0.1:5173',
    viewport: { width: 1280, height: 720 },
    video: {
      mode: 'on',
      size: { width: 1280, height: 720 },
    },
    trace: 'off',
  },
  webServer: liveBaseURL
    ? undefined
    : {
        command: 'npm run dev -- --host 127.0.0.1 --port 5173 --strictPort',
        url: 'http://127.0.0.1:5173',
        reuseExistingServer: !process.env.CI,
        timeout: 60_000,
      },
});
