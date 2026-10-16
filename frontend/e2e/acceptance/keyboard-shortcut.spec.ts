import { test, expect, type APIRequestContext, type Page } from './fixtures';
import AxeBuilder from '@axe-core/playwright';
import { mkdirSync, readFileSync } from 'node:fs';
import path from 'node:path';

const demo = JSON.parse(readFileSync('../examples/demo-v2/config.json', 'utf8'));

test.beforeEach(async ({ request }) => {
  const snapshot = await (await request.get('/api/v2/config')).json();
  expect(
    (
      await request.put('/api/v2/config', { data: { revision: snapshot.revision, config: demo } })
    ).ok(),
  ).toBe(true);
});

async function setExistingShortcut(request: APIRequestContext) {
  const snapshot = await (await request.get('/api/v2/config')).json();
  const button = snapshot.config.layout.folders[0].buttons.find(
    (item: { id: string }) => item.id === 'index-2',
  );
  button.action = { type: 'command', command: { type: 'key', keys: ['ctrl', 'c'] } };
  expect(
    (
      await request.put('/api/v2/config', {
        data: { revision: snapshot.revision, config: snapshot.config },
      })
    ).ok(),
  ).toBe(true);
}

async function openShortcutEditor(page: Page) {
  await page.goto('/');
  await page.getByRole('button', { name: 'Folder 1', exact: true }).waitFor();
  await page.keyboard.press('q');
  await page.getByRole('button', { name: 'Edit Open demo folder', exact: true }).click();
  const editor = page.locator('dialog.editor');
  await expect(page.getByRole('dialog', { name: 'Edit button', exact: true })).toBeVisible();
  await page.getByRole('tab', { name: 'Action', exact: true }).click();
  await expect(editor.getByRole('button', { name: 'Record shortcut', exact: true })).toBeVisible();
  return editor;
}

test('recording applies through the editor draft before saving the host configuration', async ({
  page,
  request,
}) => {
  await setExistingShortcut(request);
  const editor = await openShortcutEditor(page);
  await editor.getByRole('button', { name: 'Record shortcut', exact: true }).click();
  await page.keyboard.press('Control+Shift+F12');
  await expect(editor.locator('.shortcut-preview output')).toHaveText('Ctrl + Shift + F12');

  const beforeApply = await (await request.get('/api/v2/config')).json();
  expect(
    beforeApply.config.layout.folders[0].buttons.find(
      (button: { id: string }) => button.id === 'index-2',
    ).action.command.keys,
  ).toEqual(['ctrl', 'c']);

  await editor.getByRole('button', { name: 'Apply to draft', exact: true }).click();
  const afterApply = await (await request.get('/api/v2/config')).json();
  expect(
    afterApply.config.layout.folders[0].buttons.find(
      (button: { id: string }) => button.id === 'index-2',
    ).action.command.keys,
  ).toEqual(['ctrl', 'c']);

  await page.getByRole('button', { name: 'Save changes', exact: true }).click();
  await expect(page.getByText('All changes saved', { exact: true })).toBeVisible();
  const saved = await (await request.get('/api/v2/config')).json();
  expect(
    saved.config.layout.folders[0].buttons.find((button: { id: string }) => button.id === 'index-2')
      .action.command,
  ).toEqual({ type: 'key', keys: ['ctrl', 'shift', 'f12'] });
});

test('desktop picker stays outside editor layout and commits Ctrl+Alt+F24 only on Use shortcut', async ({
  page,
  request,
}, testInfo) => {
  await setExistingShortcut(request);
  await page.setViewportSize({ width: 1024, height: 900 });
  const editor = await openShortcutEditor(page);
  const geometry = async () =>
    editor.evaluate((dialog) => {
      const rect = dialog.getBoundingClientRect();
      return {
        x: rect.x,
        y: rect.y,
        width: rect.width,
        height: rect.height,
        scrollTop: dialog.scrollTop,
      };
    });
  const before = await geometry();
  const screenshotDirectory = path.resolve('..', 'docs', 'screenshots');
  mkdirSync(screenshotDirectory, { recursive: true });
  await page.screenshot({ path: path.join(screenshotDirectory, 'keyboard-shortcut-editor.png') });
  await testInfo.attach('shortcut-editor-before-picker.png', {
    body: await page.screenshot(),
    contentType: 'image/png',
  });

  await editor.getByRole('button', { name: 'Choose keys', exact: true }).click();
  const picker = page.getByRole('dialog', { name: 'Choose keys', exact: true });
  await expect(picker).toBeVisible();
  const accessibility = await new AxeBuilder({ page }).include('.picker').analyze();
  expect(accessibility.violations).toEqual([]);
  await expect(picker.getByRole('button', { name: 'Use shortcut', exact: true })).toBeInViewport();
  const during = await geometry();
  expect(Math.abs(during.width - before.width)).toBeLessThanOrEqual(1);
  expect(Math.abs(during.height - before.height)).toBeLessThanOrEqual(1);
  expect(during.scrollTop).toBe(before.scrollTop);

  const bounds = await picker.boundingBox();
  expect(bounds).not.toBeNull();
  expect(bounds!.x).toBeGreaterThanOrEqual(12);
  expect(bounds!.y).toBeGreaterThanOrEqual(12);
  expect(bounds!.x + bounds!.width).toBeLessThanOrEqual(1012);
  expect(bounds!.y + bounds!.height).toBeLessThanOrEqual(888);
  const pickerContent = picker.locator('.picker-content');
  const contentMetrics = await pickerContent.evaluate((element) => ({
    scrollHeight: element.scrollHeight,
    clientHeight: element.clientHeight,
  }));
  expect(contentMetrics.scrollHeight).toBeGreaterThan(contentMetrics.clientHeight);
  await pickerContent.evaluate((element) => (element.scrollTop = element.scrollHeight));
  expect(await pickerContent.evaluate((element) => element.scrollTop)).toBeGreaterThan(0);
  const grid = picker.locator('.key-grid');
  const gridMetrics = await grid.evaluate((element) => ({
    scrollHeight: element.scrollHeight,
    clientHeight: element.clientHeight,
  }));
  expect(gridMetrics.scrollHeight).toBeGreaterThan(gridMetrics.clientHeight);
  await grid.evaluate((element) => (element.scrollTop = 0));
  await page.screenshot({ path: path.join(screenshotDirectory, 'keyboard-shortcut-popover.png') });
  await testInfo.attach('shortcut-editor-with-picker.png', {
    body: await page.screenshot(),
    contentType: 'image/png',
  });

  const gridButtons = picker.locator('.key-grid button');
  await gridButtons.nth(0).focus();
  await page.keyboard.press('ArrowRight');
  await expect(gridButtons.nth(1)).toBeFocused();

  const pickerTrigger = editor.getByRole('button', { name: 'Choose keys', exact: true });
  await page.keyboard.press('Escape');
  await expect(picker).toHaveCount(0);
  await expect(pickerTrigger).toBeFocused();
  await expect(editor.locator('.shortcut-preview output')).toHaveText('Ctrl + C');
  await pickerTrigger.click();
  await expect(picker).toBeVisible();
  const editorHeading = await editor.locator('#button-title').boundingBox();
  await page.mouse.click(editorHeading!.x + 12, editorHeading!.y + 12);
  await expect(picker).toHaveCount(0);
  await expect(pickerTrigger).toBeFocused();
  await pickerTrigger.click();
  await expect(picker).toBeVisible();

  await picker.getByRole('button', { name: 'Alt', exact: true }).click();
  await picker.locator('.key-grid').getByRole('button', { name: /F24/ }).click();
  await expect(picker.getByRole('button', { name: 'Use shortcut', exact: true })).toBeEnabled();
  const beforeUse = await (await request.get('/api/v2/config')).json();
  expect(
    beforeUse.config.layout.folders[0].buttons.find(
      (button: { id: string }) => button.id === 'index-2',
    ).action.command.keys,
  ).toEqual(['ctrl', 'c']);
  await picker.getByRole('button', { name: 'Use shortcut', exact: true }).click();
  await expect(editor.locator('.shortcut-preview output')).toHaveText('Ctrl + Alt + F24');
  const afterClose = await geometry();
  expect(Math.abs(afterClose.width - before.width)).toBeLessThanOrEqual(1);
  expect(Math.abs(afterClose.height - before.height)).toBeLessThanOrEqual(1);
  expect(afterClose.scrollTop).toBe(before.scrollTop);

  await editor.getByRole('button', { name: 'Choose keys', exact: true }).click();
  const reopened = page.getByRole('dialog', { name: 'Choose keys', exact: true });
  await expect(reopened.locator('.selection')).toContainText('Ctrl');
  await expect(reopened.locator('.selection')).toContainText('Alt');
  await expect(reopened.locator('.selection')).toContainText('F24');
  const cancel = reopened.getByRole('button', { name: 'Cancel', exact: true });
  await cancel.click();
  await expect(editor.getByRole('button', { name: 'Choose keys', exact: true })).toBeFocused();
  await editor.getByRole('button', { name: 'Cancel', exact: true }).click();
  const confirmation = page.getByRole('dialog', { name: 'Unsaved button changes', exact: true });
  await expect(confirmation).toBeVisible();
  await confirmation.getByRole('button', { name: 'Discard changes', exact: true }).click();
  await expect(page.getByRole('dialog', { name: 'Edit button', exact: true })).toHaveCount(0);
});

test('narrow screens use an in-dialog picker step and discard cancelled candidates', async ({
  page,
  request,
}) => {
  await setExistingShortcut(request);
  await page.setViewportSize({ width: 320, height: 760 });
  const editor = await openShortcutEditor(page);

  for (const width of [320, 360, 390, 520]) {
    await page.setViewportSize({ width, height: 760 });
    await editor.getByRole('button', { name: 'Choose keys', exact: true }).click();
    await expect(editor.getByRole('heading', { name: 'Choose keys', exact: true })).toBeVisible();
    await expect(editor.getByRole('button', { name: 'Apply to draft', exact: true })).toHaveCount(
      0,
    );
    expect(await page.locator('dialog[open]').count()).toBe(1);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(
      true,
    );
    const f24 = editor.locator('.key-grid').getByRole('button', { name: /F24/ });
    await f24.click();
    await expect(editor.getByRole('button', { name: 'Use shortcut', exact: true })).toBeEnabled();
    if (width === 320) {
      await page.keyboard.press('Escape');
    } else {
      await editor.getByRole('button', { name: 'Cancel', exact: true }).click();
    }
    await expect(editor.getByRole('heading', { name: 'Edit button', exact: true })).toBeVisible();
    await expect(editor.locator('.shortcut-preview output')).toHaveText('Ctrl + C');
    await expect(editor.getByRole('tab', { name: 'Action', exact: true })).toHaveAttribute(
      'aria-selected',
      'true',
    );
  }

  for (const width of [768, 1024, 1440]) {
    await page.setViewportSize({ width, height: 760 });
    const before = await editor.boundingBox();
    await editor.getByRole('button', { name: 'Choose keys', exact: true }).click();
    const picker = page.getByRole('dialog', { name: 'Choose keys', exact: true });
    await expect(picker).toBeVisible();
    expect(await page.locator('dialog[open]').count()).toBe(1);
    const box = await picker.boundingBox();
    expect(box!.x).toBeGreaterThanOrEqual(12);
    expect(box!.x + box!.width).toBeLessThanOrEqual(width - 12);
    await picker.getByRole('button', { name: 'Cancel', exact: true }).click();
    const after = await editor.boundingBox();
    expect(Math.abs(after!.width - before!.width)).toBeLessThanOrEqual(1);
  }
  await editor.getByRole('button', { name: 'Cancel', exact: true }).click();
  await expect(page.getByRole('dialog', { name: 'Edit button', exact: true })).toHaveCount(0);
  const saved = await (await request.get('/api/v2/config')).json();
  expect(
    saved.config.layout.folders[0].buttons.find((button: { id: string }) => button.id === 'index-2')
      .action.command.keys,
  ).toEqual(['ctrl', 'c']);
});
