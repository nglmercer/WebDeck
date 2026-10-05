import { test, expect } from '@playwright/test';
import { readFileSync } from 'node:fs';
const demo = JSON.parse(readFileSync('../examples/demo-v2/config.json', 'utf8'));
test.beforeEach(async ({ request }) => {
  const snapshot = await (await request.get('/api/v2/config')).json();
  expect((await request.put('/api/v2/config', { data: { revision: snapshot.revision, config: demo } })).ok()).toBe(true);
});

test('closing or escaping an unapplied editor requires an explicit discard decision', async ({ page, request }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Folder 1', exact: true }).waitFor();
  await page.keyboard.press('q');
  await page.getByRole('button', { name: 'Edit Folder 1', exact: true }).click();
  await page.getByLabel('Label', { exact: true }).fill('My preserved folder');
  await page.getByRole('button', { name: 'Cancel', exact: true }).click();
  const confirmation = page.getByRole('dialog', { name: 'Unsaved button changes', exact: true });
  await expect(confirmation).toBeVisible();
  await confirmation.getByRole('button', { name: 'Keep editing', exact: true }).click();
  await expect(page.getByLabel('Label', { exact: true })).toHaveValue('My preserved folder');
  await page.keyboard.press('Escape');
  await expect(confirmation).toBeVisible();
  await page.keyboard.press('Escape');
  await expect(confirmation).toHaveCount(0);
  await expect(page.getByRole('dialog', { name: 'Edit button', exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Close editor', exact: true }).click();
  await confirmation.getByRole('button', { name: 'Discard changes', exact: true }).click();
  await expect(page.getByRole('dialog')).toHaveCount(0);
  expect((await (await request.get('/api/v2/config')).json()).config).toEqual(demo);
});

test('applied drafts survive folder navigation and settings until explicitly saved', async ({ page, request }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Folder 1', exact: true }).waitFor();
  await page.keyboard.press('q');
  const tools = page.getByRole('button', { name: 'Folder tools', exact: true });
  if (await tools.isVisible()) await tools.click();
  await page.getByLabel('Folder name', { exact: true }).fill('My unsaved home');
  await page.getByRole('combobox', { name: 'Folder', exact: true }).selectOption('spotify');
  await page.keyboard.press('Control+,');
  await page.getByRole('button', { name: 'Back to deck', exact: true }).click();
  if (await tools.isVisible()) await tools.click();
  await page.getByRole('combobox', { name: 'Folder', exact: true }).selectOption('index');
  await expect(page.getByLabel('Folder name', { exact: true })).toHaveValue('My unsaved home');
  expect((await (await request.get('/api/v2/config')).json()).config.layout.folders[0].label).toBe('Home');
  await page.getByRole('button', { name: 'Save changes', exact: true }).click();
  await expect(page.locator('.save-status')).toHaveText('All changes saved');
  expect((await (await request.get('/api/v2/config')).json()).config.layout.folders[0].label).toBe('My unsaved home');
});

test('touch decks retain readable labels, reachable cells and a fixed primary editing dock', async ({ page, isMobile }) => {
  test.skip(!isMobile, 'Runs in Android and iOS device profiles.');
  await page.goto('/');
  await page.getByRole('button', { name: 'Folder 1', exact: true }).waitFor();
  const tile = page.locator('[data-cell="0"] > .deck-button');
  const before = await tile.boundingBox();
  await page.getByRole('button', { name: 'Edit', exact: true }).click();
  expect(await tile.boundingBox()).toEqual(before);
  const dock = page.locator('.editor-toolbar');
  await expect(page.getByRole('button', { name: 'Done', exact: true })).toBeInViewport();
  expect(await dock.evaluate(el => el.scrollHeight <= el.clientHeight)).toBe(true);
  await page.getByRole('button', { name: 'Folder tools', exact: true }).click();
  await page.locator('#folder-tools').evaluate(el => el.scrollTop = el.scrollHeight);
  await expect(page.getByRole('button', { name: 'Done', exact: true })).toBeInViewport();
  await page.getByRole('button', { name: 'Folder tools', exact: true }).click();
  await page.getByRole('button', { name: 'Done', exact: true }).click();
  await page.locator('.deck-scroll').evaluate(el => el.scrollTop = el.scrollHeight);
  await expect(page.getByRole('button', { name: 'GPU usage', exact: true })).toBeInViewport();
  const labels = await page.locator('.buttontext').evaluateAll(elements => elements.map(el => ({ font: parseFloat(getComputedStyle(el).fontSize), clipped: el.scrollHeight > el.clientHeight + 1, text: el.textContent })));
  expect(labels.every(label => label.font >= 12)).toBe(true);
  expect(labels.filter(label => label.clipped)).toEqual([]);
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
});
