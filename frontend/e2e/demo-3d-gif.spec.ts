import { promises as fs } from 'node:fs';
import * as path from 'node:path';
import { fileURLToPath } from 'node:url';
import { expect, test, type Page } from '@playwright/test';
import { setupDemoMocks } from './demo/mocks';

// Animated tour GIF frames: camera flights between angles plus two app
// screens (grid + Spotify folder). Assembled by finalize-3d-gif.mjs.
const HERE = path.dirname(fileURLToPath(import.meta.url));
const FRAMES_DIR = path.join(HERE, '..', 'demo', '.frames-3d');

let frame = 0;

async function snap(page: Page): Promise<void> {
  frame += 1;
  await page.screenshot({ path: path.join(FRAMES_DIR, `f${String(frame).padStart(3, '0')}.png`) });
}

async function hold(page: Page, n: number): Promise<void> {
  for (let i = 0; i < n; i++) {
    await page.waitForTimeout(120);
    await snap(page);
  }
}

async function flyTo(page: Page, angle: string, n: number): Promise<void> {
  await page.evaluate((name) => window.__demo3d?.setAngle(name), angle);
  for (let i = 0; i < n; i++) {
    await page.waitForTimeout(120);
    await snap(page);
  }
}

test.setTimeout(300_000);
test('3d tour gif frames', async ({ page }) => {
  await setupDemoMocks(page);
  await page.goto('/demo-3d/?clean=1');
  await page.waitForFunction(() => window.__demo3d?.ready === true, null, {
    timeout: 30_000,
  });
  const app = page.frameLocator('.wd3d-panel iframe');
  await app.locator('[data-testid="deck-tile"]').first().waitFor({ timeout: 30_000 });
  await page.evaluate(() => window.__demo3d?.setAutoRotate(false));
  await fs.rm(FRAMES_DIR, { recursive: true, force: true });
  await fs.mkdir(FRAMES_DIR, { recursive: true });

  await page.evaluate(() => window.__demo3d?.setAngle('cover'));
  await page.waitForTimeout(1000);
  await hold(page, 3);
  await flyTo(page, 'front', 7);
  await hold(page, 2);
  await flyTo(page, 'hero', 7);

  // Second app screen: dive into the Spotify folder, then back out.
  await app.locator('#folder-index [data-message="/folder spotify"]').click();
  await app.locator('.buttons-center#folder-spotify').waitFor({ timeout: 15_000 });
  await hold(page, 4);
  await app.locator('#folder-spotify [data-message="/folder index"]').click();
  await app.locator('.buttons-center#folder-index').waitFor({ timeout: 15_000 });
  await hold(page, 2);

  await flyTo(page, 'side', 7);
  await flyTo(page, 'cover', 7);
  await hold(page, 3);

  expect(frame).toBeGreaterThan(30);
});
