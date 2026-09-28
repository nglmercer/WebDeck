import { defineConfig } from '@playwright/test';

// Tour-GIF frame capture (see e2e/demo-3d-gif.spec.ts); assembled by
// e2e/finalize-3d-gif.mjs into demo/3d-tour.gif.
export default defineConfig({
  testDir: './e2e',
  testMatch: 'demo-3d-gif.spec.ts',
  fullyParallel: false,
  workers: 1,
  reporter: 'list',
  outputDir: './e2e/test-results',
  use: {
    baseURL: 'http://127.0.0.1:5173',
    // 1x scale: frames downscale to 960px for the GIF, so retina
    // source only slows the 40+ screenshots down.
    viewport: { width: 1920, height: 1080 },
    deviceScaleFactor: 1,
  },
  webServer: {
    command: 'npm run dev -- --host 127.0.0.1 --port 5173 --strictPort',
    url: 'http://127.0.0.1:5173',
    reuseExistingServer: !process.env.CI,
    timeout: 60_000,
  },
});
