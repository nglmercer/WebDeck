import { test, expect, type Page } from './fixtures';
import AxeBuilder from '@axe-core/playwright';
import { readFileSync } from 'node:fs';
const guides = JSON.parse(
  readFileSync(new URL('../../src/features/learn/guide-data.json', import.meta.url), 'utf8'),
) as Guide[];
import type { Guide } from '../../src/features/learn/guides';

async function library(page: Page) {
  await page.keyboard.press('F1');
  await page.getByRole('button', { name: 'Learn WebDeck', exact: true }).click();
  await expect(page.getByRole('dialog', { name: 'Learn WebDeck', exact: true })).toBeVisible();
}

async function traverse(page: Page, guide: Guide, interact = false) {
  for (const [index, step] of guide.steps.entries()) {
    const popover = page.locator(
      `.driver-popover[data-guide="${guide.id}"][data-guide-step="${step.id}"]`,
    );
    await expect(popover, `${guide.id}/${step.id}`).toBeVisible();
    const target = page.locator('.driver-active-element');
    await expect(target).toHaveCount(1);
    await expect(target).toBeVisible();
    expect(
      await target.evaluate((element, selector) => element.matches(selector), step.target),
    ).toBe(true);
    expect(await page.locator('dialog:modal').count()).toBe(0);
    const bounds = (await popover.boundingBox())!;
    const viewport = page.viewportSize()!;
    expect(bounds.x).toBeGreaterThanOrEqual(0);
    expect(bounds.y).toBeGreaterThanOrEqual(0);
    expect(bounds.x + bounds.width).toBeLessThanOrEqual(viewport.width + 1);
    expect(bounds.y + bounds.height).toBeLessThanOrEqual(viewport.height + 1);
    await expect(popover.locator('.driver-popover-progress-text')).toHaveText(
      `${index + 1} of ${guide.steps.length}`,
    );
    if (interact && guide.id === 'content' && step.id === 'label')
      await target.locator('input').fill('Unsaved tutorial example');
    if (interact && guide.id === 'keyboard') {
      if (step.id === 'record') {
        await target.click();
        await page.keyboard.press('Control+Shift+F12');
      }
      if (step.id === 'modifiers')
        await target.getByRole('button', { name: 'Alt', exact: true }).click();
      if (step.id === 'search') await target.getByRole('searchbox').fill('F24');
      if (step.id === 'keys')
        await target.getByRole('button', { name: 'F24', exact: true }).click();
    }
    if (interact && guide.id === 'audio' && step.id === 'volume')
      await target.getByRole('spinbutton').fill('25');
    if (interact && guide.id === 'practice' && step.interactive) await target.click();
    await popover.locator('.driver-popover-next-btn').click();
  }
  await expect(page.locator('.driver-popover')).toHaveCount(0);
  await expect
    .poll(() => page.evaluate(() => document.body.classList.contains('driver-active')))
    .toBe(false);
}

for (const guide of guides.filter((guide) => guide.access !== 'pairing')) {
  test(`guide ${guide.id} reaches every audited surface without changing the host`, async ({
    page,
    request,
  }) => {
    await page.emulateMedia({ reducedMotion: 'reduce' });
    await page.setViewportSize({ width: 1440, height: 1000 });
    const errors: string[] = [],
      writes: string[] = [];
    page.on('pageerror', (error) => errors.push(error.message));
    page.on('request', (request) => {
      if (request.url().includes('/api/') && !['GET', 'HEAD'].includes(request.method()))
        writes.push(`${request.method()} ${request.url()}`);
    });
    const before = await (await request.get('/api/v2/config')).json();
    await page.goto('/');
    await page.getByRole('button', { name: 'Play / pause', exact: true }).waitFor();
    const url = page.url();
    await library(page);
    await page.locator(`[data-guide-id="${guide.id}"]`).click();
    await traverse(page, guide, true);
    expect(await (await request.get('/api/v2/config')).json()).toEqual(before);
    expect(writes).toEqual([]);
    expect(errors).toEqual([]);
    expect(page.url()).toBe(url);
    await expect
      .poll(() =>
        page.evaluate(() => JSON.parse(localStorage.getItem('webdeck.guides.v1') ?? '[]')),
      )
      .toContain(guide.id);
    if (guide.id !== 'practice') await expect(page.locator('.deck-scroll')).toBeFocused();
  });
}

test('contextual editor help preserves unapplied input, supports back and restores native dialog focus', async ({
  page,
}) => {
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto('/');
  await page.getByRole('button', { name: 'Play / pause', exact: true }).waitFor();
  await page.keyboard.press('q');
  await page.getByRole('button', { name: 'Edit Play / pause', exact: true }).click();
  const editor = page.locator('dialog.editor');
  await editor.getByLabel('Label', { exact: true }).fill('Keep my unapplied label');
  await editor.getByRole('button', { name: 'Guide: Edit button', exact: true }).click();
  await expect(page.locator('[data-guide-step="label"].driver-popover')).toBeVisible();
  await page.getByRole('button', { name: 'Next', exact: true }).click();
  await expect(page.locator('[data-guide-step="preview"].driver-popover')).toBeVisible();
  await page.getByRole('button', { name: 'Previous', exact: true }).click();
  await expect(page.locator('[data-guide-step="label"].driver-popover')).toBeVisible();
  await page.keyboard.press('Tab');
  expect(
    await page.evaluate(
      () =>
        !!document.activeElement?.closest('.driver-popover') ||
        !!document.activeElement?.closest('[data-guide="button-label"]'),
    ),
  ).toBe(true);
  await page.keyboard.press('Escape');
  await expect(page.locator('.driver-popover')).toHaveCount(0);
  await expect(editor.getByLabel('Label', { exact: true })).toHaveValue('Keep my unapplied label');
  expect(await editor.evaluate((element) => element.matches(':modal'))).toBe(true);
  await expect(
    editor.getByRole('button', { name: 'Guide: Edit button', exact: true }),
  ).toBeFocused();
  expect(
    await page.evaluate(() => JSON.parse(localStorage.getItem('webdeck.guides.v1') ?? '[]')),
  ).not.toContain('content');
});

test('pairing guide teaches the request and code flow without creating a request', async ({
  page,
}) => {
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.route('**/api/v2/boot', (route) =>
    route.fulfill({
      status: 401,
      json: { error: { code: 'unauthorized', message: 'Pair this controller' } },
    }),
  );
  const writes: string[] = [];
  page.on('request', (request) => {
    if (request.url().includes('/api/') && !['GET', 'HEAD'].includes(request.method()))
      writes.push(request.url());
  });
  await page.goto('/');
  await page.getByLabel('Device name', { exact: true }).fill('My pending phone');
  await page.getByRole('button', { name: 'Guide: Connect your device', exact: true }).click();
  await traverse(
    page,
    guides.find((guide) => guide.id === 'pairing')!,
  );
  await expect(page.getByLabel('Device name', { exact: true })).toHaveValue('My pending phone');
  expect(writes).toEqual([]);
});

test('guide library filters, replays completion and stays accessible on a narrow screen', async ({
  page,
}) => {
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.setViewportSize({ width: 375, height: 812 });
  await page.goto('/');
  await page.getByRole('button', { name: 'Play / pause', exact: true }).waitFor();
  await library(page);
  const learn = page.getByRole('dialog', { name: 'Learn WebDeck', exact: true });
  await learn
    .getByRole('combobox', { name: 'Guide category', exact: true })
    .selectOption('buttons');
  await learn.getByRole('searchbox', { name: 'Find a guide', exact: true }).fill('audio');
  await expect(learn.locator('.lessons .lesson')).toHaveCount(1);
  await learn.locator('[data-guide-id="audio"]').click();
  await expect(page.locator('.driver-popover')).toBeVisible();
  const accessibility = await new AxeBuilder({ page }).include('.driver-popover').analyze();
  expect(accessibility.violations).toEqual([]);
  await traverse(
    page,
    guides.find((guide) => guide.id === 'audio')!,
  );
  await library(page);
  await expect(learn.locator('[data-guide-id="audio"] .completed')).toHaveText('Completed');
  await learn.locator('[data-guide-id="audio"]').click();
  await expect(page.locator('[data-guide-step="source"].driver-popover')).toBeVisible();
  await page.getByRole('button', { name: 'Close tour', exact: true }).click();
  await expect(page.locator('.driver-popover')).toHaveCount(0);
});
