import { test, expect, type Page } from './fixtures';
async function openLearning(page: Page) {
  await page.keyboard.press('F1');
  await page.getByRole('button', { name: 'Learn WebDeck', exact: true }).click();
}

test('guided tour supports navigation, escape, replay and focus restoration', async ({ page }, info) => {
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto('/');
  await openLearning(page);
  await page.getByRole('button', { name: /Take a guided tour/ }).click();
  await expect(page.locator('.driver-popover-title')).toHaveText('Your deck');
  await page.screenshot({ path: info.outputPath('guided-tour.png'), fullPage: true });
  await page.getByRole('button', { name: 'Next', exact: true }).click();
  await expect(page.locator('.driver-popover-title')).toHaveText('Move between folders');
  await page.getByRole('button', { name: 'Previous', exact: true }).click();
  await expect(page.locator('.driver-popover-title')).toHaveText('Your deck');
  await page.keyboard.press('Escape');
  await expect(page.locator('.driver-popover')).toHaveCount(0);
  await expect(page.locator('.deck-scroll')).toBeFocused();
  await openLearning(page);
  await page.getByRole('button', { name: /Take a guided tour/ }).click();
  for (let i = 0; i < 4; i++) await page.getByRole('button', { name: 'Next', exact: true }).click();
  await expect(page.locator('.driver-popover-title')).toHaveText('Keep exploring');
  await page.getByRole('button', { name: 'Finish tour', exact: true }).click();
  await expect(page.locator('.driver-popover')).toHaveCount(0);
  await expect(page.locator('.deck-scroll')).toBeFocused();
});

test('practice controls are isolated, resettable and fit a mobile screen', async ({ page, request }, info) => {
  const before = await (await request.get('/api/v2/config')).json();
  const mutations: string[] = [];
  page.on('request', req => { if (req.url().includes('/api/') && !['GET', 'HEAD'].includes(req.method())) mutations.push(req.url()); });
  await page.setViewportSize({ width: 375, height: 812 });
  await page.goto('/');
  await openLearning(page);
  await page.getByRole('button', { name: /Try the interactive demo/ }).click();
  const demo = page.getByRole('dialog', { name: 'Practice deck', exact: true });
  await demo.getByRole('button', { name: /Break scene/ }).click();
  await expect(demo.getByRole('status')).toHaveText('Scene switched to Break scene.');
  await demo.getByRole('button', { name: /Microphone/ }).click();
  await expect(demo.getByRole('button', { name: /Microphone/ })).toHaveAttribute('aria-pressed', 'false');
  await demo.getByRole('button', { name: /Recording/ }).click();
  await expect(demo.getByRole('status')).toHaveText('Recording: On.');
  await demo.getByRole('button', { name: 'Reset demo' }).click();
  await expect(demo.getByRole('button', { name: /Recording/ })).toHaveAttribute('aria-pressed', 'false');
  expect(await demo.evaluate(el => el.scrollWidth <= el.clientWidth)).toBe(true);
  await page.screenshot({ path: info.outputPath('practice-mobile.png'), fullPage: true });
  await page.keyboard.press('Escape');
  await expect(demo).toHaveCount(0);
  expect(mutations).toEqual([]);
  expect(await (await request.get('/api/v2/config')).json()).toEqual(before);
  for (const width of [375, 768, 1440]) {
    await page.setViewportSize({ width, height: 900 });
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
    await page.screenshot({ path: info.outputPath(`workspace-${width}.png`), fullPage: true });
  }
});

test('viewer tour omits editing and connection steps and keeps shortcuts isolated', async ({ page }) => {
  await page.route('**/api/v2/boot', async route => {
    const response = await route.fetch();
    const boot = await response.json();
    await route.fulfill({ response, json: { ...boot, can_edit: false } });
  });
  await page.goto('/');
  await openLearning(page);
  await page.getByRole('button', { name: /Take a guided tour/ }).click();
  await expect(page.locator('.driver-popover-progress-text')).toHaveText('1 of 3');
  for (let i = 0; i < 2; i++) await page.getByRole('button', { name: 'Next', exact: true }).click();
  await expect(page.locator('.driver-popover-title')).toHaveText('Keep exploring');
  await page.getByRole('button', { name: 'Finish tour', exact: true }).click();
  await expect(page.locator('.editor-toolbar')).toHaveCount(0);
});

test('tray tutorial link opens learning on settings and back navigation clears the link', async ({page, request}) => {
  const before = await (await request.get('/api/v2/config')).json();
  const mutations: string[] = [];
  page.on('request', req => {if (req.url().includes('/api/') && !['GET', 'HEAD'].includes(req.method())) mutations.push(req.url());});
  await page.goto('/?view=settings&section=tutorials');
  const library = page.getByRole('dialog', {name: 'Learn WebDeck', exact: true});
  await expect(library).toBeVisible();
  await library.getByRole('button', {name: 'Close', exact: true}).click();
  await expect(page.getByRole('heading', {name: 'Settings', exact: true})).toBeVisible();
  await page.getByRole('button', {name: 'Back to deck', exact: true}).click();
  await expect(page.locator('.deck-scroll')).toBeVisible();
  expect(await page.evaluate(() => location.search)).toBe('');
  await page.reload();
  await expect(page.locator('.deck-scroll')).toBeVisible();
  await expect(library).toHaveCount(0);
  expect(mutations).toEqual([]);
  expect(await (await request.get('/api/v2/config')).json()).toEqual(before);
});

test('settings query links select sections and legacy settings links still work', async ({page}) => {
  for (const [section, name] of [['devices', 'Devices & Connections'], ['integrations', 'Integrations'], ['backups', 'Backups'], ['runtime', 'Runtime and plugins']]) {
    await page.goto(`/?view=settings&section=${section}`);
    await expect(page.getByRole('tab', {name, exact: true})).toHaveAttribute('aria-selected', 'true');
    await expect(page.getByRole('tabpanel', {name, exact: true})).toBeVisible();
  }
  await page.goto('/#settings');
  await expect(page.getByRole('heading', {name: 'Settings', exact: true})).toBeVisible();
});

test('a viewer cannot open privileged settings from a tray query link', async ({page}) => {
  await page.route('**/api/v2/boot', async route => {
    const response = await route.fetch();
    await route.fulfill({response, json: {...await response.json(), can_edit: false}});
  });
  await page.goto('/?view=settings&section=tutorials');
  await expect(page.locator('.deck-scroll')).toBeVisible();
  await expect(page.getByRole('heading', {name: 'Settings', exact: true})).toHaveCount(0);
  await expect(page.getByRole('dialog', {name: 'Learn WebDeck', exact: true})).toHaveCount(0);
});
