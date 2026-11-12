import { test, expect } from './fixtures';
import AxeBuilder from '@axe-core/playwright';

const tabs = ['Appearance', 'Integrations', 'Devices & Connections', 'Backups', 'Runtime and plugins'];
test('flat settings adapt every section to desktop and mobile without hidden controls', async ({ page }, info) => {
  await page.goto('/#settings');
  await expect(page.getByRole('heading', { name: 'Settings', exact: true })).toBeVisible();
  for (const width of [1440, 390, 360]) {
    await page.setViewportSize({ width, height: 960 });
    for (const [index, tab] of tabs.entries()) {
      await page.getByRole('tab', { name: tab, exact: true }).click();
      const panel = page.getByRole('tabpanel');
      await expect(panel).toBeVisible();
      expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
      await expect(page.locator('.settings-shell select, .settings-shell details, .settings-shell summary')).toHaveCount(0);
      const flat = await panel.evaluate(el => [...el.querySelectorAll('section, button, input')].every(node => getComputedStyle(node).boxShadow === 'none'));
      expect(flat).toBe(true);
      await page.screenshot({ path: info.outputPath(`settings-${width}-${index}.png`), fullPage: true });
    }
  }
  await page.getByRole('tab', { name: 'Appearance', exact: true }).click();
  const results = await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa', 'wcag21aa']).analyze();
  expect(results.violations).toEqual([]);
});

test('steppers and language radio cards stage changes before saving', async ({ page, request }) => {
  const original = await (await request.get('/api/v2/config')).json();
  await page.goto('/#settings');
  const columns = page.getByLabel('Columns', { exact: true });
  await expect(columns).toHaveValue(String(original.config.layout.columns));
  await page.getByRole('button', { name: 'Increase Columns', exact: true }).click();
  await expect(columns).toHaveValue(String(original.config.layout.columns + 1));
  await columns.fill('');
  await expect(page.getByRole('button', { name: 'Save changes', exact: true })).toBeDisabled();
  await columns.fill(String(original.config.layout.columns + 1));
  await page.locator('input[type=radio][value=es_ES]').check();
  expect(await (await request.get('/api/v2/config')).json()).toEqual(original);
  await page.getByRole('button', { name: 'Save changes', exact: true }).click();
  await expect(page.locator('#settings-save-status')).toHaveText('Todos los cambios están guardados');
  const saved = await (await request.get('/api/v2/config')).json();
  expect(saved.config.layout.columns).toBe(original.config.layout.columns + 1);
  expect(saved.config.settings.language).toBe('es_ES');
});


test('help dialog follows the flat theme and restores keyboard focus', async ({ page }) => {
  await page.goto('/#settings');
  const help = page.getByRole('button', { name: 'Help and tutorials', exact: true });
  await help.click();
  const dialog = page.getByRole('dialog', { name: 'Learn WebDeck', exact: true });
  await expect(dialog).toBeVisible();
  await expect(dialog).toHaveCSS('box-shadow', 'none');
  await page.keyboard.press('Escape');
  await expect(dialog).toHaveCount(0);
  await expect(help).toBeFocused();
});


test('invalid network port cannot be applied', async ({ page, request }) => {
  const before = await (await request.get('/api/v2/network')).json();
  await page.goto('/#settings');
  await page.getByRole('tab', { name: 'Devices & Connections', exact: true }).click();
  const port = page.getByRole('spinbutton', { name: 'Phone connection port', exact: true });
  await expect(port).toHaveValue(String(before.settings.port));
  await port.fill('70000');
  await expect(page.getByRole('button', { name: 'Apply phone access', exact: true })).toBeDisabled();
  expect((await (await request.get('/api/v2/network')).json()).settings).toEqual(before.settings);
  await port.fill(String(before.settings.port));
  await expect(page.getByRole('button', { name: 'Apply phone access', exact: true })).toBeEnabled();
});
