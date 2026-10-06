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
  await page.getByRole('button', { name: 'Folder 1', exact: true }).click();
  await page.getByRole('button', { name: 'Rename folder', exact: true }).click();
  await page.getByLabel('Folder name', { exact: true }).fill('My unsaved folder');
  await page.getByLabel('Folder name', { exact: true }).press('Enter');
  await page.keyboard.press('Control+,');
  await page.getByRole('button', { name: 'Back to deck', exact: true }).click();
  await page.keyboard.press('Alt+ArrowLeft');
  await page.getByRole('button', { name: 'My unsaved folder', exact: true }).click();
  await expect(page.locator('.folder-title')).toHaveText('My unsaved folder');
  expect((await (await request.get('/api/v2/config')).json()).config.layout.folders[1].label).toBe('Folder 1');
  await page.getByRole('button', { name: 'Save changes', exact: true }).click();
  await expect(page.locator('.save-status')).toHaveText('All changes saved');
  await expect(page.getByRole('button', { name: 'Save changes', exact: true })).toHaveCount(0);
  expect((await (await request.get('/api/v2/config')).json()).config.layout.folders[1].label).toBe('My unsaved folder');
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
  await page.getByRole('button', { name: 'Deck actions', exact: true }).click();
  await expect(page.getByRole('menuitem', { name: 'New folder', exact: true })).toBeVisible();
  await page.keyboard.press('Escape');
  await expect(page.getByRole('button', { name: 'Done', exact: true })).toBeInViewport();
  await page.getByRole('button', { name: 'Done', exact: true }).click();
  await page.locator('.deck-scroll').evaluate(el => el.scrollTop = el.scrollHeight);
  await expect(page.getByRole('button', { name: 'GPU usage', exact: true })).toBeInViewport();
  const labels = await page.locator('.buttontext').evaluateAll(elements => elements.map(el => ({ font: parseFloat(getComputedStyle(el).fontSize), clipped: el.scrollHeight > el.clientHeight + 1, text: el.textContent })));
  expect(labels.every(label => label.font >= 12)).toBe(true);
  expect(labels.filter(label => label.clipped)).toEqual([]);
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
});

test('icon picker fits narrow screens and clears selection for empty searches', async ({ page }) => {
  await page.setViewportSize({ width: 375, height: 740 });
  await page.goto('/');
  await page.getByRole('button', { name: 'Folder 1', exact: true }).waitFor();
  await page.keyboard.press('q');
  await page.getByRole('button', { name: 'Edit Folder 1', exact: true }).click();
  const trigger = page.getByRole('button', { name: 'Icon', exact: true });
  await trigger.click();
  const picker = page.getByRole('dialog', { name: 'Select icon', exact: true });
  await expect(picker).toBeVisible();
  for (const width of [375, 1280]) {
    await page.setViewportSize({ width, height: 740 });
    expect(await picker.evaluate(el => el.scrollWidth <= el.clientWidth)).toBe(true);
    const box = await picker.boundingBox();
    expect(box!.x).toBeGreaterThanOrEqual(0);
    expect(box!.x + box!.width).toBeLessThanOrEqual(width);
  }
  await picker.getByLabel('Search icons', { exact: true }).fill('zzzz-no-such-icon');
  await expect(picker.getByRole('status')).toHaveText('No icons match your search.');
  await expect(picker.getByRole('button', { name: 'Select', exact: true })).toBeDisabled();
  await picker.getByLabel('Search icons', { exact: true }).fill('folder');
  await picker.getByRole('option', { name: 'folder', exact: true }).click();
  await picker.getByRole('button', { name: 'Select', exact: true }).click();
  await expect(picker).toHaveCount(0);
  await expect(trigger).toBeFocused();
  await trigger.click();
  await page.keyboard.press('Escape');
  await expect(picker).toHaveCount(0);
  await expect(page.getByRole('dialog', { name: 'Edit button', exact: true })).toBeVisible();
});

test('custom image sources preserve drafts when switching methods on mobile', async ({ page }) => {
  await page.setViewportSize({ width: 375, height: 800 });
  await page.goto('/');
  await page.getByRole('button', { name: 'Folder 1', exact: true }).waitFor();
  await page.keyboard.press('q');
  await page.getByRole('button', { name: 'Edit Folder 1', exact: true }).click();
  await page.getByText('Use a custom image', { exact: true }).click();
  const editor = page.getByRole('dialog', { name: 'Edit button', exact: true });
  const sources = editor.getByRole('group', { name: 'Image source', exact: true });
  await expect(editor.getByRole('button', { name: 'Choose an image' })).toBeVisible();
  await sources.getByRole('button', { name: 'Image URL', exact: true }).click();
  await expect(editor.getByRole('button', { name: 'Import image URL', exact: true })).toBeDisabled();
  await editor.getByLabel('Image URL', { exact: true }).fill('https://example.com/test.png');
  await sources.getByRole('button', { name: 'Local file', exact: true }).click();
  await expect(editor.getByRole('button', { name: 'Choose local image', exact: true })).toBeVisible();
  await expect(editor.getByRole('button', { name: 'Import local image', exact: true })).toBeDisabled();
  await sources.getByRole('button', { name: 'Image URL', exact: true }).click();
  await expect(editor.getByLabel('Image URL', { exact: true })).toHaveValue('https://example.com/test.png');
  await expect(editor.getByRole('button', { name: 'Import image URL', exact: true })).toBeEnabled();
  expect(await editor.evaluate(el => el.scrollWidth <= el.clientWidth)).toBe(true);
});

test('minimal action picker preserves configuration on cancel and handles empty searches', async ({ page }) => {
  await page.setViewportSize({ width: 375, height: 800 });
  await page.goto('/');
  await page.getByRole('button', { name: 'Folder 1', exact: true }).waitFor();
  await page.keyboard.press('q');
  await page.getByRole('button', { name: 'Edit Folder 1', exact: true }).click();
  await page.getByRole('tab', { name: 'Action', exact: true }).click();
  await expect(page.getByRole('combobox', { name: 'Destination', exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Change action', exact: true }).click();
  const picker = page.getByRole('dialog', { name: 'Select action', exact: true });
  await picker.getByLabel('Find an action', { exact: true }).fill('zzzz-no-action');
  await expect(picker.getByRole('status')).toBeVisible();
  await expect(picker.getByRole('button', { name: 'Select action', exact: true })).toBeDisabled();
  await picker.getByRole('button', { name: 'Cancel', exact: true }).click();
  await expect(page.getByRole('combobox', { name: 'Destination', exact: true })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Change action', exact: true })).toBeFocused();
  await page.getByRole('button', { name: 'Change action', exact: true }).click();
  await picker.getByLabel('Find an action', { exact: true }).fill('Play / pause');
  await picker.locator('.option').filter({ hasText: 'Play / pause' }).click();
  expect(await picker.evaluate(el => el.scrollWidth <= el.clientWidth)).toBe(true);
  await picker.getByRole('button', { name: 'Select action', exact: true }).click();
  await expect(page.getByText('No configuration needed.', { exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Change action', exact: true }).click();
  await picker.getByLabel('Find an action', { exact: true }).fill('Type text');
  await picker.locator('.option').filter({ hasText: 'Type text' }).click();
  await picker.getByRole('button', { name: 'Select action', exact: true }).click();
  await expect(page.getByLabel('Text to type', { exact: true })).toBeVisible();
  await expect(page.getByText('No configuration needed.', { exact: true })).toHaveCount(0);
});
