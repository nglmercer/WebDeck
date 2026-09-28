import { promises as fs } from 'node:fs';
import * as path from 'node:path';
import { fileURLToPath } from 'node:url';
import { expect, test } from '@playwright/test';
import { setupDemoMocks } from './demo/mocks';

// 3D showcase stills: deterministic backend mocks (shared with the video
// tour), scripted camera angles, one PNG per angle in frontend/demo/.
const HERE = path.dirname(fileURLToPath(import.meta.url));
const DEMO_DIR = path.join(HERE, '..', 'demo');
const ANGLES = ['cover', 'front', 'hero', 'side', 'top'] as const;

test.setTimeout(120_000);
test('3d showcase captures', async ({ page }) => {
  await setupDemoMocks(page);
  await page.goto('/demo-3d/?clean=1');
  await page.waitForFunction(() => window.__demo3d?.ready === true, null, {
    timeout: 30_000,
  });
  // Live app inside the 3D panel must be up before the first shot.
  const app = page.frameLocator('.wd3d-panel iframe');
  await app.locator('[data-testid="deck-tile"]').first().waitFor({ timeout: 30_000 });
  await expect.poll(async () => page.evaluate(() => window.__demo3d?.ready)).toBe(true);
  await page.evaluate(() => window.__demo3d?.setAutoRotate(false));
  await fs.mkdir(DEMO_DIR, { recursive: true });

  for (const angle of ANGLES) {
    await page.evaluate((name) => window.__demo3d?.setAngle(name), angle);
    await page.waitForTimeout(1000);
    await page.screenshot({ path: path.join(DEMO_DIR, `3d-${angle}.png`) });
  }
});
