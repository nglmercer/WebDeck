import { test, expect } from '@playwright/test';
import { readFileSync } from 'node:fs';
const demo = JSON.parse(readFileSync('../examples/demo-v2/config.json', 'utf8'));
test.beforeEach(async ({ request }) => {
  const snapshot = await (await request.get('/api/v2/config')).json();
  expect((await request.put('/api/v2/config', { data: { revision: snapshot.revision, config: demo } })).ok()).toBe(true);
});

test('master demo preserves cells, icons and folder history at desktop and mobile sizes', async ({ page }, info) => {
  const errors: string[] = [];
  page.on('pageerror', error => errors.push(error.message));
  await page.goto('/');
  await page.getByRole('button', { name: 'Got it', exact: true }).click();
  await expect(page.locator('.deck-cell')).toHaveCount(32);
  await expect(page.locator('[data-cell="24"] button')).toHaveAccessibleName('Settings');
  for (const width of [360, 768, 1440]) {
    await page.setViewportSize({ width, height: 900 });
    await expect.poll(async () => page.locator('.deck-cell').last().evaluate(element => element.getBoundingClientRect().right <= window.innerWidth)).toBe(true);
    expect(await page.locator('.deck-button img').evaluateAll(images => images.every(image => (image as HTMLImageElement).complete && (image as HTMLImageElement).naturalWidth > 0))).toBe(true);
    await page.screenshot({ path: info.outputPath(`master-demo-${width}.png`), fullPage: true });
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  }
  await page.getByRole('button', { name: 'Spotify', exact: true }).click();
  await expect(page).toHaveURL(/#folder=spotify$/);
  await expect(page.getByRole('button', { name: 'Play/Pause', exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Back', exact: true }).click();
  await expect(page).toHaveURL(/#folder=index$/);
  await page.goBack();
  await expect(page).toHaveURL(/#folder=spotify$/);
  await expect(page.getByRole('button', { name: 'Next', exact: true })).toBeVisible();
  await page.keyboard.press('Alt+ArrowLeft');
  await expect(page.getByRole('button', { name: 'Settings', exact: true })).toBeVisible();
  expect(errors).toEqual([]);
});

test('an invalid catalog keeps the deck and settings usable', async ({ page }) => {
  await page.route('**/api/v2/commands', route => route.fulfill({ json: { api_version: 2, commands: 'invalid', plugins: [] } }));
  await page.goto('/');
  await expect(page.getByRole('alert')).toContainText('Invalid CatalogResponse');
  await expect(page.getByRole('button', { name: 'Folder 1', exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Settings', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Back to deck', exact: true })).toBeVisible();
});

test('a failed boot offers recovery instead of an empty screen', async ({ page }) => {
  await page.route('**/api/v2/boot', route => route.fulfill({ status: 503, json: { message: 'Unavailable' } }));
  await page.goto('/');
  await expect(page.getByRole('heading', { name: 'WebDeck could not load', exact: true })).toBeVisible();
  await page.unroute('**/api/v2/boot');
  await page.getByRole('button', { name: 'Reload', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Folder 1', exact: true })).toBeVisible();
});
