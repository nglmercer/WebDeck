import { test, expect } from './fixtures';
import { readFileSync } from 'node:fs';
const demo = JSON.parse(readFileSync('../examples/demo-v2/config.json', 'utf8'));

test('deck uses the viewport with no permanent header, titles, panel or dock', async ({ page, request }, info) => {
  const snapshot = await (await request.get('/api/v2/config')).json();
  expect((await request.put('/api/v2/config', { data: { revision: snapshot.revision, config: demo } })).ok()).toBe(true);
  for (const size of [{ width: 1440, height: 900 }, { width: 375, height: 812 }]) {
    await page.setViewportSize(size);
    await page.goto('/');
    const deck = page.getByRole('region', { name: 'Control deck', exact: true });
    await expect(deck).toBeVisible();
    await expect(page.locator('header, nav, .editor-toolbar, .deck-heading, .workspace-intro, [role="note"]')).toHaveCount(0);
    expect(await page.locator('h1').evaluate(el => el.getBoundingClientRect().width)).toBe(1);
    const box = await deck.boundingBox();
    expect(box!.y).toBeLessThanOrEqual(12);
    expect(box!.width).toBeGreaterThanOrEqual(size.width - 24);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
    await page.screenshot({ path: info.outputPath(`minimal-${size.width}.png`), fullPage: true });
  }
  await page.keyboard.press('F1');
  await page.getByRole('tab', { name: 'Deck controls', exact: true }).click();
  await page.getByRole('button', { name: 'Edit', exact: true }).click();
  await expect(page.locator('.editor-toolbar')).toBeVisible();
  await page.getByRole('button', { name: 'Done', exact: true }).click();
  await expect(page.locator('.editor-toolbar')).toHaveCount(0);
});

test('only Settings is protected from deletion in both editing entry points', async ({ page, request }) => {
  await page.goto('/');
  await page.locator('.deck-scroll').waitFor();
  await page.keyboard.press('q');
  await expect(page.getByRole('button', { name: 'Remove Settings', exact: true })).toHaveCount(0);
  await expect(page.getByRole('button', { name: 'Remove Play / pause', exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Edit Settings', exact: true }).click();
  const dialog = page.getByRole('dialog', { name: 'Edit button', exact: true });
  await dialog.getByLabel('More button actions', { exact: true }).click();
  await expect(dialog.getByRole('button', { name: 'Delete', exact: true })).toBeDisabled();
  await dialog.getByRole('tab', { name: 'Action', exact: true }).click();
  await expect(dialog.getByRole('button', { name: 'Change action', exact: true })).toBeDisabled();
  await dialog.getByRole('button', { name: 'Cancel', exact: true }).click();
  await page.getByRole('button', { name: 'Remove Play / pause', exact: true }).click();
  await page.getByRole('button', { name: 'Save changes', exact: true }).click();
  await expect(page.locator('.save-status')).toHaveText('All changes saved');
  const saved = await (await request.get('/api/v2/config')).json();
  expect(saved.config.layout.folders.flatMap((folder: { buttons: { action: { type: string } }[] }) => folder.buttons).some((button: { action: { type: string } }) => button.action.type === 'settings')).toBe(true);
});
