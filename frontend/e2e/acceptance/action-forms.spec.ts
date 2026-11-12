import { test, expect, type Page, type APIRequestContext } from './fixtures';
import AxeBuilder from '@axe-core/playwright';
import type { Command } from '../../src/lib/contracts';

async function seed(page: Page, request: APIRequestContext, command: Command, language = 'en_US') {
  const snapshot = await (await request.get('/api/v2/config')).json();
  snapshot.config.settings.language = language;
  snapshot.config.layout.folders[0].buttons[0].action = { type: 'command', command };
  expect(
    (
      await request.put('/api/v2/config', {
        data: { revision: snapshot.revision, config: snapshot.config },
      })
    ).ok(),
  ).toBe(true);
  const original = await (await request.get('/api/v2/config')).json();
  await page.goto('/');
  await page.getByRole('button', { name: 'Play / pause', exact: true }).waitFor();
  expect(
    await page.evaluate(() =>
      performance
        .getEntriesByType('resource')
        .some((entry) => /\/assets\/editor-/.test(entry.name)),
    ),
  ).toBe(false);
  await page.keyboard.press('q');
  await page
    .getByRole('button', {
      name: language === 'es_ES' ? 'Editar Play / pause' : 'Edit Play / pause',
      exact: true,
    })
    .click();
  await page
    .getByRole('tab', { name: language === 'es_ES' ? 'Acción' : 'Action', exact: true })
    .click();
  return { original, editor: page.locator('dialog.editor') };
}

const http: Command = {
  type: 'fetch',
  method: 'GET',
  url: 'https://example.com',
  headers: { 'Content-Type': 'application/json' },
  body: '{}',
  timeout_seconds: 15,
};

test('HTTP key/value rows preserve typing, block duplicate headers and save only after Apply and Save', async ({
  page,
  request,
}, info) => {
  await page.setViewportSize({ width: 1440, height: 1000 });
  const { original, editor } = await seed(page, request, http);
  const errors: string[] = [];
  page.on('pageerror', (error) => errors.push(error.message));
  const method = editor.getByRole('combobox', { name: 'Method', exact: true });
  await method.click();
  await page.getByRole('option', { name: 'POST', exact: true }).click();
  await editor.getByRole('button', { name: 'Add header', exact: true }).click();
  const key = editor.getByRole('textbox', { name: 'Name 2', exact: true });
  await key.pressSequentially('Authorization');
  await expect(key).toBeFocused();
  await editor.getByRole('textbox', { name: 'Value 2', exact: true }).fill('Bearer example');
  await key.fill('content-type');
  await editor.getByRole('button', { name: 'Apply to draft', exact: true }).click();
  await expect(editor).toBeVisible();
  await expect(editor.getByRole('alert').first()).toContainText('unique');
  await key.fill('Authorization');
  const body = editor.getByRole('textbox', { name: 'Body', exact: true });
  await expect(body).toHaveAttribute('contenteditable', 'true');
  await body.fill('{"event":"live","active":true}');
  await expect(editor.locator('.cm-gutters')).toBeVisible();
  await expect(editor.locator('.cm-content span').first()).toBeVisible();
  expect(await (await request.get('/api/v2/config')).json()).toEqual(original);
  await editor.screenshot({ path: info.outputPath('http-editor.png') });
  await editor.getByRole('button', { name: 'Apply to draft', exact: true }).click();
  await expect(editor).toHaveCount(0);
  expect(await (await request.get('/api/v2/config')).json()).toEqual(original);
  await page.getByRole('button', { name: 'Save changes', exact: true }).click();
  const saved = await (await request.get('/api/v2/config')).json();
  expect(saved.config.layout.folders[0].buttons[0].action.command).toEqual({
    ...http,
    method: 'POST',
    headers: { 'Content-Type': 'application/json', Authorization: 'Bearer example' },
    body: '{"event":"live","active":true}',
  });
  expect(errors).toEqual([]);
});

test('script editor highlights safely, preserves code when changing source and reuses the upload card', async ({
  page,
  request,
}, info) => {
  const { original, editor } = await seed(page, request, {
    type: 'script',
    source: { type: 'inline', code: 'const before = 1;' },
  });
  const code = editor.getByRole('textbox', { name: 'Code', exact: true });
  await expect(code).toHaveAttribute('contenteditable', 'true');
  const text = 'const message = "<script>literal</script>";\nconsole.log(message);';
  await code.fill(text);
  expect(await page.evaluate(() => document.querySelectorAll('script:not([src])').length)).toBe(0);
  await editor.getByRole('button', { name: 'Expand code editor', exact: true }).click();
  await expect(editor.locator('.code-field')).toHaveClass(/expanded/);
  await editor.getByRole('button', { name: 'Collapse code editor', exact: true }).click();
  await editor.getByRole('radio', { name: 'File', exact: true }).check();
  await expect(editor.locator('.file-picker')).toBeVisible();
  await expect(editor.locator('textarea:not(.proxy)')).toHaveCount(0);
  await editor.getByRole('radio', { name: 'Code', exact: true }).check();
  await expect
    .poll(() => editor.getByRole('textbox', { name: 'Code', exact: true }).innerText())
    .toBe(text);
  await editor.screenshot({ path: info.outputPath('script-code.png') });
  await editor.getByRole('radio', { name: 'File', exact: true }).check();
  await editor.locator('.file-picker input[type=file]').setInputFiles({
    name: 'shortcut.js',
    mimeType: 'text/javascript',
    buffer: Buffer.from('console.log("file");'),
  });
  await expect(editor.locator('.file-copy strong')).toHaveText('shortcut.js');
  await editor.screenshot({ path: info.outputPath('script-file.png') });
  expect(await (await request.get('/api/v2/config')).json()).toEqual(original);
  await editor.getByRole('button', { name: 'Apply to draft', exact: true }).click();
  await page.getByRole('button', { name: 'Save changes', exact: true }).click();
  const saved = await (await request.get('/api/v2/config')).json();
  const command = saved.config.layout.folders[0].buttons[0].action.command;
  expect(command.type).toBe('script');
  expect(command.source.type).toBe('file');
  expect(command.source.source.type).toBe('asset');
  expect((await request.get('/api/v2/assets/' + command.source.source.id)).ok()).toBe(true);
  await page.reload();
  await page.getByRole('button', { name: 'Play / pause', exact: true }).waitFor();
  await page.keyboard.press('q');
  await page.getByRole('button', { name: 'Edit Play / pause', exact: true }).click();
  await page.getByRole('tab', { name: 'Action', exact: true }).click();
  await expect(editor.locator('.file-copy strong')).toHaveText('shortcut.js');
});

test('application selector loads native options, supports keyboard and custom targets, and volume uses a slider', async ({
  page,
  request,
}, info) => {
  let queries = 0;
  await page.route('**/api/v2/applications?kind=audio', (route) => {
    queries++;
    return route.fulfill({
      json: {
        api_version: 2,
        available: true,
        applications: [
          { name: 'Spotify', value: 'spotify', description: 'Music', executable: '' },
          { name: 'OBS Studio', value: 'obs', description: 'Recording', executable: '' },
        ],
      },
    });
  });
  const { original, editor } = await seed(page, request, {
    type: 'app_volume',
    application: 'previous-app',
    change: { type: 'set', percent: 75 },
  });
  const combo = editor.getByRole('combobox', { name: 'Application', exact: true });
  await expect(combo).toContainText('previous-app');
  await combo.click();
  const search = page.getByRole('searchbox', { name: 'Search options', exact: true });
  await search.fill('OBS');
  await search.press('Enter');
  await expect(combo).toContainText('OBS Studio');
  await expect(combo).toBeFocused();
  await combo.click();
  await search.fill('Spotify');
  await search.press('Tab');
  await expect(combo).toHaveAttribute('aria-expanded', 'false');
  await expect(editor.getByRole('button', { name: 'Refresh options', exact: true })).toBeFocused();
  await combo.click();
  await search.fill('');
  await search.press('Home');
  await search.press('ArrowDown');
  await editor.screenshot({ path: info.outputPath('application-options.png') });
  await search.press('Enter');
  await expect(combo).toContainText('OBS Studio');
  const slider = editor.getByRole('slider', { name: 'Volume', exact: true });
  await slider.focus();
  await page.keyboard.press('Home');
  await page.keyboard.press('ArrowRight');
  await expect(editor.getByRole('spinbutton', { name: 'Volume', exact: true })).toHaveValue('1');
  await editor.getByRole('radio', { name: 'Adjust', exact: true }).check();
  await expect(slider).toHaveAttribute('min', '-100');
  await editor.getByRole('spinbutton', { name: 'Volume', exact: true }).fill('-15');
  await combo.click();
  await page.getByRole('searchbox', { name: 'Search options', exact: true }).fill('custom-player');
  await page.getByRole('option', { name: 'Use “custom-player”', exact: true }).click();
  await expect(combo).toContainText('custom-player');
  await editor.getByRole('button', { name: 'Refresh options', exact: true }).click();
  await expect.poll(() => queries).toBe(2);
  await expect(combo).toContainText('custom-player');
  await editor.screenshot({ path: info.outputPath('application-volume.png') });
  expect(await (await request.get('/api/v2/config')).json()).toEqual(original);
  await editor.getByRole('button', { name: 'Apply to draft', exact: true }).click();
  await page.getByRole('button', { name: 'Save changes', exact: true }).click();
  const saved = await (await request.get('/api/v2/config')).json();
  expect(saved.config.layout.folders[0].buttons[0].action.command).toEqual({
    type: 'app_volume',
    application: 'custom-player',
    change: { type: 'adjust', percent: -15 },
  });
});

test('mobile Spanish forms have translated labels, contained popovers, switches and accessible code editing', async ({
  page,
  request,
}, info) => {
  await page.setViewportSize({ width: 375, height: 812 });
  const { editor } = await seed(page, request, http, 'es_ES');
  await expect(editor.getByRole('combobox', { name: 'Método', exact: true })).toBeVisible();
  await editor.getByRole('combobox', { name: 'Método', exact: true }).click();
  const popup = page.locator('.select-popup');
  const bounds = (await popup.boundingBox())!;
  expect(bounds.x).toBeGreaterThanOrEqual(0);
  expect(bounds.x + bounds.width).toBeLessThanOrEqual(376);
  await page.keyboard.press('Escape');
  await expect(editor).toBeVisible();
  await expect(editor.getByRole('textbox', { name: 'Cuerpo', exact: true })).toHaveAttribute(
    'contenteditable',
    'true',
  );
  const results = await new AxeBuilder({ page })
    .include('dialog.editor')
    .withTags(['wcag2a', 'wcag2aa', 'wcag21aa'])
    .analyze();
  expect(results.violations).toEqual([]);
  await editor.screenshot({ path: info.outputPath('http-spanish-mobile.png') });
  await editor.getByRole('tab', { name: 'Apariencia', exact: true }).click();
  const toggle = editor.getByRole('switch', { name: 'Mostrar etiqueta', exact: true });
  await expect(toggle).toBeChecked();
  await toggle.uncheck();
  await expect(toggle).not.toBeChecked();
  expect(await editor.locator('input[type=checkbox]:not([role=switch])').count()).toBe(0);
});
