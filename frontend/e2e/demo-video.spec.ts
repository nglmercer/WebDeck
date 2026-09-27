import { promises as fs } from 'node:fs';
import * as path from 'node:path';
import { fileURLToPath } from 'node:url';
import { expect, test } from '@playwright/test';

// Full-tour demo recording (720p). Default mode mocks the Rust backend so
// the shoot is deterministic and triggers no real button actions on the
// host. Live mode (WEBDECK_DEMO_BASE_URL set) records a real server as-is.
const LIVE = Boolean(process.env.WEBDECK_DEMO_BASE_URL);
const HERE = path.dirname(fileURLToPath(import.meta.url));
const FIXTURES = path.join(HERE, 'fixtures');
const DEMO_DIR = path.join(HERE, '..', 'demo');
const DEMO_VIDEO = path.join(DEMO_DIR, 'webdeck-demo-720p.webm');

async function readJson(name: string): Promise<unknown> {
  return JSON.parse(await fs.readFile(path.join(FIXTURES, name), 'utf8'));
}

test.setTimeout(180_000);
test('webdeck demo tour', async ({ page }) => {
  // Save flows use alert(); accept so the tour never stalls.
  page.on('dialog', (dialog) => void dialog.accept());

  if (!LIVE) {
    const boot = (await readJson('boot.json')) as { config: unknown };
    const usage = await readJson('usage.json');

    await page.route('**/api/boot', (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify(boot) })
    );
    // Editor enter uses GET, boot uses POST: same config payload either way.
    await page.route('**/get_config', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify(boot.config),
      })
    );
    await page.route('**/usage', (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify(usage) })
    );
    for (const endpoint of [
      '**/send-data',
      '**/save_config',
      '**/COMPLETE_save_config',
      '**/save_single_button',
      '**/save_buttons_only',
    ]) {
      await page.route(endpoint, (route) =>
        route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify({ success: true, message: 'demo mode' }),
        })
      );
    }
    // Vite dev does not serve the Rust-owned /static tree; serve the repo
    // copy so button icons and CSS render in the recording.
    await page.route('**/static/**', (route) => {
      const file = path.join(
        HERE,
        '..',
        '..',
        new URL(route.request().url()).pathname.replace(/^\/+/, '')
      );
      return route.fulfill({ path: file }).catch(() => route.fulfill({ status: 404 }));
    });
  }

  // 1. Boot: loading screen -> grid, usage tiles fill in.
  await page.goto('/');
  await expect(page.locator('#button_e0X0')).toBeVisible();
  await expect(page.locator('#folder-index .usage-value').first()).not.toHaveText('-', {
    timeout: 15_000,
  });
  await page.waitForTimeout(1500);

  // 2. Folder navigation: index -> spotify -> index.
  await page.locator('#button_e0X1').click();
  await expect(page.locator('.buttons-center#folder-spotify')).toBeVisible();
  await page.waitForTimeout(1200);
  await page.locator('#button_e2X0').click();
  await expect(page.locator('.buttons-center#folder-index')).toBeVisible();
  await page.waitForTimeout(1000);

  // 3. Settings tile opens the config modal, close button dismisses it.
  await page.locator('#button_e0X24').click();
  await expect(page.locator('.modal-container')).toBeVisible();
  await page.waitForTimeout(1500);
  await page.locator('.modal-close').click();
  await expect(page.locator('.modal-container')).toBeHidden();
  await page.waitForTimeout(800);

  // 4. Editor mode, rename a button, save (reloads back into editor mode).
  await page.keyboard.press('q');
  await expect(page.locator('#EditorButtons')).toBeVisible();
  await page.waitForTimeout(1000);
  await page.locator('.edit-button[edit_modal_ID="e0X0"]').click();
  await expect(page.locator('#edit-modal-container-e0X0')).toBeVisible();
  await page.waitForTimeout(1000);
  await page.locator('#button-text-input_e0X0').fill('My Media Folder');
  await page.waitForTimeout(1000);
  await page.locator('#e0X0_submit').click();
  await page.waitForURL('**/*edit=true*', { timeout: 15_000 });
  await expect(page.locator('#EditorButtons')).toBeVisible({ timeout: 15_000 });
  await page.waitForTimeout(1000);

  // 5. Save & exit the editor, then press a command button.
  await page.locator('#SaveExitEditorButton').click();
  await expect(page.locator('#EditorButtons')).toBeHidden();
  await page.waitForTimeout(1000);
  await page.locator('#button_e0X6').click();
  // Let the usage poll repopulate the tiles (reload restarted it) before closing.
  await expect(page.locator('#folder-index .usage-value').first()).not.toHaveText('-', {
    timeout: 15_000,
  });
  await page.waitForTimeout(1500);

  // Publish the recording at a stable path.
  const video = page.video();
  expect(video).not.toBeNull();
  await page.close();
  await fs.mkdir(DEMO_DIR, { recursive: true });
  await video?.saveAs(DEMO_VIDEO);
});
