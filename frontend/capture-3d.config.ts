import { defineConfig } from '@playwright/test';

// 3D showcase capture setup (see e2e/demo-3d.spec.ts). Kept separate from
// the video tour config so `demo:video` stays a single-tour run.
export default defineConfig({
  testDir: './e2e',
  testMatch: 'demo-3d.spec.ts',
  fullyParallel: false,
  workers: 1,
  reporter: 'list',
  outputDir: './e2e/test-results',
  use: {
    baseURL: 'http://127.0.0.1:5173',
    viewport: { width: 1920, height: 1080 },
    deviceScaleFactor: 2,
  },
  webServer: {
    command: 'npm run dev -- --host 127.0.0.1 --port 5173 --strictPort',
    url: 'http://127.0.0.1:5173',
    reuseExistingServer: !process.env.CI,
    timeout: 60_000,
  },
});
