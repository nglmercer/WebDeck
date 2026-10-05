import { test, expect, chromium, type Page, type TestInfo, type BrowserContext } from '@playwright/test';
import { io } from 'socket.io-client';
import { readFileSync, mkdtempSync, mkdirSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import type { CommandEvent } from '../../src/lib/contracts';
const initial = JSON.parse(readFileSync('../webdeck/config_default.json', 'utf8'));
initial.layout.folders[0].buttons.push({
  id: 'work-link',
  label: 'Work folder',
  icon: '▦',
  color: '#6654e8',
  action: { type: 'folder', folder_id: 'work' },
  extensions: {},
});
initial.layout.folders.push({ id: 'work', label: 'Work', buttons: [], extensions: {} });
async function cancelButtonEditor(page: Page, label = 'Cancel') {
  await page.getByRole('button', { name: label, exact: true }).click();
  const confirm = page.getByRole('dialog', { name: /Unsaved button changes|Cambios del botón sin aplicar/ });
  if (await confirm.isVisible()) await confirm.getByRole('button', { name: /Discard changes|Descartar cambios/ }).click();
}
async function captureState(page: Page, info: TestInfo, state: string) {
  for (const width of [360, 768, 1440]) {
    await page.setViewportSize({ width, height: 900 });
    await page.screenshot({ path: info.outputPath(`${state}-${width}.png`), fullPage: true });
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
  }
}
test.beforeEach(async ({ request }) => {
  const s = await (await request.get('/api/v2/config')).json();
  expect(
    (await request.put('/api/v2/config', { data: { revision: s.revision, config: initial } })).ok(),
  ).toBeTruthy();
});
test('new deck edits typed actions, folders, themes and keeps conflict drafts', async ({
  page,
  request,
}, testInfo) => {
  const errors: string[] = [];
  page.on('pageerror', (e) => errors.push(e.message));
  await page.goto('/');
  await expect(page.getByRole('heading', { name: 'Home', exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Work folder', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Work', exact: true })).toBeVisible();
  await page.keyboard.press('Alt+ArrowLeft');
  await page.keyboard.press('q');
  await page.getByRole('button', { name: 'Edit Play / pause' }).click();
  await page.getByLabel('Label', { exact: true }).fill('My music');
  await page.getByRole('button', { name: 'Apply to draft' }).click();
  await page.getByRole('button', { name: /^Save( changes)?$/, exact: true }).click();
  await expect(page.getByRole('status').filter({ hasText: /^Saved/ })).toBeVisible();
  await page.reload();
  await expect(page.getByRole('button', { name: 'My music', exact: true })).toBeVisible();
  await page.keyboard.press('q');
  await page.getByRole('button', { name: 'Edit My music' }).click();
  await page.getByLabel('Label', { exact: true }).fill('Unsaved draft');
  await page.getByRole('button', { name: 'Apply to draft' }).click();
  const latest = await (await request.get('/api/v2/config')).json();
  latest.config.layout.rows = 4;
  expect(
    (
      await request.put('/api/v2/config', {
        data: { revision: latest.revision, config: latest.config },
      })
    ).ok(),
  ).toBeTruthy();
  await page.getByRole('button', { name: /^Save( changes)?$/, exact: true }).click();
  await expect(page.getByRole('alert')).toContainText('draft');
  await expect(page.getByRole('button', { name: 'Edit Unsaved draft' })).toBeVisible();
  await captureState(page, testInfo, 'conflict');
  expect(errors).toEqual([]);
});
test('theme and image uploads, folder creation and settings round trip', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Settings', exact: true }).first().click();
  await page.getByLabel('Upload a theme').setInputFiles({
    name: 'theme.css',
    mimeType: 'text/css',
    buffer: Buffer.from('.deck-grid{--test-theme:loaded}.deck-button{border-top:3px solid rgb(1,2,3)}'),
  });
  const image = {
    name: 'image.png',
    mimeType: 'image/png',
    buffer: Buffer.from(
      'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+jK1cAAAAASUVORK5CYII=',
      'base64',
    ),
  };
  await page.getByLabel('Upload a background').setInputFiles(image);
  await page.getByRole('tab', { name: 'Appearance', exact: true }).click();
  await page.getByLabel('Columns', { exact: true }).fill('3');
  await page.getByRole('button', { name: /^Save( changes)?$/, exact: true }).click();
  await expect(page.getByRole('status').filter({ hasText: /^Saved/ })).toBeVisible();
  await page.reload();
  await page.getByRole('button', { name: 'Back to deck' }).click();
  await expect(page.locator('.deck-grid')).toHaveCSS('--test-theme', 'loaded');
  await expect(page.locator('.deck-button').first()).toHaveCSS('border-top-width', '3px');
  await page.keyboard.press('q');
  await page.getByRole('button', { name: /Add folder/ }).click();
  await page.getByLabel('Folder name').fill('Studio');
  await page.getByRole('button', { name: /^Save( changes)?$/, exact: true }).click();
  await page.reload();
  await expect(page.getByRole('heading', { name: 'Studio', exact: true })).toBeVisible();
  await page.keyboard.press('q');
  await page.getByRole('button', { name: /Add button in cell/ }).first().click();
  await page.getByLabel('Label', { exact: true }).fill('Image button');
  await page.getByLabel('Upload image').setInputFiles(image);
  await expect(page.getByLabel('Icon', { exact: true })).toHaveValue(/^asset:/);
  await page.getByRole('button', { name: 'Apply to draft' }).click();
  await expect(page.getByRole('button', { name: 'Image button', exact: true }).locator('img')).toBeVisible();
  const unsaved = await (await page.request.get('/api/v2/config')).json();
  expect(unsaved.config.layout.folders.flatMap((folder: { buttons: { label: string }[] }) => folder.buttons)
    .some((button: { label: string }) => button.label === 'Image button')).toBe(false);
  await page.getByRole('button', { name: /^Save( changes)?$/ }).click();
  await page.reload();

  await expect(
    page.getByRole('button', { name: 'Image button', exact: true }).locator('img'),
  ).toBeVisible();
});
test('typed realtime correlation, retired routes, revoked sessions and offline commands', async ({
  page,
  request,
}, testInfo) => {
  const socket = io('http://127.0.0.1:59996/v2', { auth: {}, reconnection: false });
  try {
    await new Promise<void>((resolve, reject) => {
      socket.once('connect', () => resolve());
      socket.once('connect_error', reject);
    });
    const events = await new Promise<CommandEvent[]>((resolve, reject) => {
      const events: CommandEvent[] = [];
      const timer = setTimeout(() => reject(Error('No terminal result')), 5000);
      socket.on('command_result', (e: CommandEvent) => {
        if (e.request_id !== 'correlation') return;
        events.push(e);
        if (e.state !== 'accepted') {
          clearTimeout(timer);
          resolve(events);
        }
      });
      socket.emit('command', { request_id: 'correlation', command: { type: 'play_pause' } });
    });
    expect(events.map((e) => e.state)).toEqual(['accepted', 'completed']);
  } finally {
    socket.disconnect();
  }
  await page.goto('/');
  await expect(page.getByRole('heading', { name: 'Home', exact: true })).toBeVisible();
  await page.keyboard.press('Control+,');
  await page.getByRole('tab', { name: 'Connection', exact: true }).click();
  await page.locator('#settings-connection select').selectOption('socket');
  await expect(page.getByLabel('Realtime connection', { exact: true })).toHaveText('Connected');
  await page.getByRole('button', { name: 'Back to deck' }).click();
  await page.getByRole('button', { name: 'Play / pause', exact: true }).click();
  await expect(page.getByRole('status')).toContainText('Completed');
  await page.context().setOffline(true);
  await page.getByRole('button', { name: 'Play / pause', exact: true }).click();
  await expect(page.getByRole('alert')).toBeVisible();
  for (const width of [360, 768, 1440]) {
    await page.setViewportSize({ width, height: 900 });
    const hint = await page.getByRole('note', { name: 'Deck tips' }).boundingBox();
    const alert = await page.getByRole('alert').boundingBox();
    expect(hint).not.toBeNull();
    expect(alert).not.toBeNull();
    expect(hint!.y + hint!.height).toBeLessThanOrEqual(alert!.y);
  }
  await captureState(page, testInfo, 'offline');
  await page.context().setOffline(false);
  expect((await request.post('/send-data', { data: { message: '/key x' } })).status()).toBe(404);
  expect((await request.post('/api/v2/commands', { data: { message: '/key x' } })).status()).toBe(
    422,
  );
});
test('paired controller never receives integration settings and revocation rejects further reads', async ({
  page,
  request,
}) => {
  const grant = await (
    await request.post('/api/v2/devices', {
      data: { name: 'Acceptance controller', capabilities: ['read', 'input'], ttl_seconds: 60 },
    })
  ).json();
  const headers = { Authorization: `Bearer ${grant.token}` };
  const boot = await (await request.get('/api/v2/boot', { headers })).json();
  expect(boot.settings).toBeUndefined();
  expect(boot.can_edit).toBe(false);
  expect(boot.layout.folders[0].buttons[0].action.command).toEqual({
    type: 'button',
    button_id: 'media',
  });
  expect(
    (
      await request.post('/api/v2/commands', {
        headers,
        data: { request_id: 'denied-button', command: { type: 'button', button_id: 'media' } },
      })
    ).status(),
  ).toBe(403);
  expect((await request.get('/api/v2/config', { headers })).status()).toBe(403);
  await page.addInitScript((token) => sessionStorage.setItem('webdeck.device', token), grant.token);
  await page.goto('/');
  await expect(page.getByRole('heading', { name: 'Home', exact: true })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Edit deck', exact: true })).toHaveCount(0);
  expect((await request.delete(`/api/v2/devices/${grant.device.id}`)).ok()).toBeTruthy();
  expect((await request.get('/api/v2/boot', { headers })).status()).toBe(401);
  await page.reload();
  await expect(page.getByRole('heading', { name: 'Connect your device' })).toBeVisible();
});
test('live usage returns typed metrics and the UI presents memory and CPU', async ({
  page,
  request,
}) => {
  const r = await request.get('/api/v2/usage');
  expect(r.ok()).toBeTruthy();
  const v = await r.json();
  expect(v.api_version).toBe(2);
  expect(v.usage.memory_total).toBeGreaterThan(0);
  expect(Array.isArray(v.usage.gpus)).toBe(true);
  await page.goto('/');
  await expect(page.locator('header, aside, footer')).toHaveCount(0);
  const cpu = page.getByRole('button', { name: 'CPU', exact: true });
  await expect(cpu.locator('.metric-value')).toContainText('%');
  await expect(cpu.getByRole('progressbar')).toHaveCount(1);
  await expect(
    page.getByRole('button', { name: 'Memory', exact: true }).locator('.metric-value'),
  ).toContainText('%');
  await expect(page.getByRole('heading', { name: 'System usage' })).toHaveCount(0);
});

test('empty decks retain hidden editing access and new folders get navigation buttons', async ({
  page,
  request,
}, testInfo) => {
  const snapshot = await (await request.get('/api/v2/config')).json();
  snapshot.config.layout.folders = [{ id: 'empty', label: 'Empty', buttons: [], extensions: {} }];
  expect(
    (
      await request.put('/api/v2/config', {
        data: { revision: snapshot.revision, config: snapshot.config },
      })
    ).ok(),
  ).toBeTruthy();
  await page.goto('/');
  await expect(page.getByRole('heading', { name: 'Empty', exact: true })).toBeVisible();
  await expect(page.locator('.deck-grid button')).toHaveCount(0);
  await captureState(page, testInfo, 'empty');
  await page.keyboard.press('q');
  await page.getByRole('button', { name: '+ Add folder', exact: true }).click();
  await page.getByLabel('Folder name').fill('Nested');
  await page.getByRole('button', { name: 'Save changes', exact: true }).click();
  await expect(page.locator('.save-status')).toHaveText('All changes saved');
  await page.keyboard.press('q');
  await expect(page.locator('.deck-grid')).not.toHaveClass(/editing/);
  await page.getByRole('button', { name: 'Back', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Nested', exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Nested', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Nested', exact: true })).toBeVisible();
  await page.mouse.click(1, 1, { button: 'right' });
  await expect(page.getByRole('dialog', { name: 'Deck controls' })).toBeVisible();
});

test('touch hold opens controls without invoking the held button', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Play / pause', exact: true }).waitFor();
  let executions = 0;
  page.on('request', (r) => {
    if (r.method() === 'POST' && r.url().endsWith('/commands')) executions++;
  });
  await page
    .getByRole('button', { name: 'Play / pause', exact: true })
    .dispatchEvent('pointerdown', { pointerType: 'touch', clientX: 50, clientY: 50 });
  await page.dispatchEvent('.deck-grid', 'pointermove', {
    pointerType: 'touch',
    clientX: 53,
    clientY: 51,
  });
  await expect(page.getByRole('dialog', { name: 'Deck controls' })).toBeVisible();
  await page.dispatchEvent('.deck-grid', 'pointerup', { pointerType: 'touch' });
  expect(executions).toBe(0);
});

test('Q edits fixed cells, F1 shows shortcuts, and removing preserves neighboring positions', async ({page}) => {
  await page.goto('/');
  await page.getByRole('button', {name:'Play / pause', exact:true}).waitFor();
  const before = await page.getByRole('button', {name:'CPU', exact:true}).locator('..').getAttribute('data-cell');
  await page.keyboard.press('F1');
  await expect(page.getByRole('dialog', {name:'Keyboard shortcuts'})).toContainText('Q');
  await page.keyboard.press('Escape');
  await page.keyboard.press('q');
  await page.getByRole('button', {name:'Remove Settings', exact:true}).click();
  await expect(page.getByRole('button', {name:'CPU', exact:true}).locator('..')).toHaveAttribute('data-cell', before!);
  await page.keyboard.press('q');
  await page.reload();
  await page.getByRole('button', {name:'CPU', exact:true}).waitFor();
  await expect(page.getByRole('button', {name:'CPU', exact:true}).locator('..')).toHaveAttribute('data-cell', before!);
  await page.keyboard.press('q');
  await page.getByRole('button', {name:'Add button in cell 2', exact:true}).click();
  await page.getByLabel('Label', {exact:true}).fill('Replacement');
  await page.getByRole('button', {name:'Apply to draft'}).click();
  await expect(page.getByRole('button', {name:'Replacement', exact:true}).locator('..')).toHaveAttribute('data-cell','1');
});

test('cancelled placement edits leave the draft and neighboring cells unchanged', async ({ page }) => {
  await page.goto('/');
  const settings = page.locator('.deck-grid').getByRole('button', { name: 'Settings', exact: true });
  const before = await settings.locator('..').getAttribute('data-cell');
  await page.keyboard.press('q');
  await page.getByRole('button', { name: 'Edit Play / pause' }).click();
  await page.getByRole('button', { name: 'Move right', exact: true }).click();
  await cancelButtonEditor(page);
  await expect(settings.locator('..')).toHaveAttribute('data-cell', before!);
  await page.keyboard.press('q');
  await page.reload();
  await expect(settings.locator('..')).toHaveAttribute('data-cell', before!);
});

test('malformed folder hashes recover and the Edit action opens editing', async ({ page, request }) => {
  const snapshot = await (await request.get('/api/v2/config')).json();
  snapshot.config.layout.folders[0].buttons.push({ id: 'edit-action', label: 'Edit deck', icon: '✎', color: '#242424', action: { type: 'edit' }, extensions: {} });
  await request.put('/api/v2/config', { data: { revision: snapshot.revision, config: snapshot.config } });
  const errors: string[] = [];
  page.on('pageerror', (error) => errors.push(error.message));
  await page.goto('/#folder=%zz');
  await expect(page.getByRole('button', { name: 'Play / pause', exact: true })).toBeVisible();
  const edit = page.getByRole('button', { name: 'Edit deck', exact: true });
  await edit.click();
  await expect(page.getByRole('button', { name: 'Edit Play / pause' })).toBeVisible();
  expect(errors).toEqual([]);
});

for (const width of [360, 768, 1440]) {
  test(`core views fit a ${width}px viewport`, async ({ page }, testInfo) => {
    await page.setViewportSize({ width, height: 900 });
    await page.goto('/');
    await expect(page.getByRole('button', { name: 'Play / pause', exact: true })).toBeVisible();
    await page.screenshot({ path: testInfo.outputPath('deck.png'), fullPage: true });
    await page.keyboard.press('q');
    await page.screenshot({ path: testInfo.outputPath('editing.png'), fullPage: true });
    await page.getByRole('button', { name: 'Edit Play / pause' }).click();
    await page.screenshot({ path: testInfo.outputPath('button-editor.png'), fullPage: true });
    await cancelButtonEditor(page);
    await page.keyboard.press('Control+,');
    await expect(page.getByRole('heading', { name: 'Appearance', exact: true })).toBeVisible();
    await page.screenshot({ path: testInfo.outputPath('settings.png'), fullPage: true });
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
  });
}

test('edits made during a pending save remain dirty and can be saved afterward', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Play / pause', exact: true }).waitFor();
  await page.keyboard.press('q');
  await page.getByLabel('Folder name', { exact: true }).fill('Saved version');
  let release!: () => void;
  let observed!: () => void;
  const gate = new Promise<void>((resolve) => release = resolve);
  const sent = new Promise<void>((resolve) => observed = resolve);
  await page.route('**/api/v2/config', async (route) => {
    if (route.request().method() !== 'PUT') return route.continue();
    const response = await route.fetch();
    observed();
    await gate;
    await route.fulfill({ response });
  });
  await page.getByRole('button', { name: 'Save changes', exact: true }).click();
  await sent;
  await expect(page.getByRole('button', { name: 'Save changes', exact: true })).toHaveAccessibleDescription('Saving…');
  await expect(page.getByRole('button', { name: 'Done', exact: true })).toHaveAccessibleDescription('Saving…');
  await page.getByLabel('Folder name', { exact: true }).fill('Later edit');
  release();
  await expect(page.getByLabel('Save status', { exact: true })).toHaveText('Unsaved changes');
  await expect(page.getByLabel('Folder name', { exact: true })).toHaveValue('Later edit');
  await page.unroute('**/api/v2/config');
  await page.getByRole('button', { name: 'Save changes', exact: true }).click();
  await expect(page.getByLabel('Save status', { exact: true })).toHaveText('All changes saved');
  await expect(page.getByRole('button', { name: 'Save changes', exact: true })).toHaveAccessibleDescription('All changes saved');
  await page.reload();
  await expect(page.getByRole('heading', { name: 'Later edit', exact: true })).toBeVisible();
});

test('deck control dialogs contain keyboard focus and Escape restores the deck', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Play / pause', exact: true }).focus();
  await page.keyboard.press('F1');
  const dialog = page.getByRole('dialog', { name: 'Keyboard shortcuts' });
  await expect(dialog).toBeVisible();
  await page.keyboard.press('Tab');
  expect(await dialog.evaluate((element) => element.contains(document.activeElement))).toBe(true);
  await page.keyboard.press('Escape');
  await expect(dialog).toHaveCount(0);
  await expect(page.getByRole('button', { name: 'Play / pause', exact: true })).toBeFocused();
});

test('core screens have no automated WCAG A/AA violations', async ({ page }) => {
  const { default: AxeBuilder } = await import('@axe-core/playwright');
  const review = async () => {
    const result = await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa']).analyze();
    expect(result.violations.map(({ id, nodes }) => ({ id, targets: nodes.map((node) => node.target) }))).toEqual([]);
  };
  await page.goto('/');
  await page.getByRole('button', { name: 'Play / pause', exact: true }).waitFor();
  await review();
  await page.keyboard.press('q');
  await review();
  await page.getByRole('button', { name: 'Edit Play / pause' }).click();
  await review();
  await cancelButtonEditor(page);
  await page.keyboard.press('Control+,');
  await review();
});

test('paired settings grants cannot retrieve action source or integration credentials', async ({ request }) => {
  const grant = await (await request.post('/api/v2/devices', { data: { name: 'Restricted settings grant', capabilities: ['read', 'settings'], ttl_seconds: 60 } })).json();
  const headers = { Authorization: `Bearer ${grant.token}` };
  const snapshot = await (await request.get('/api/v2/config')).json();
  expect((await request.get('/api/v2/config', { headers })).status()).toBe(403);
  expect((await request.put('/api/v2/config', { headers, data: { revision: snapshot.revision, config: snapshot.config } })).status()).toBe(403);
  const boot = await (await request.get('/api/v2/boot', { headers })).json();
  expect(boot.can_edit).toBe(false);
  expect(boot.settings).toBeUndefined();
  expect(boot.layout.folders[0].buttons[0].action.command).toEqual({ type: 'button', button_id: 'media' });
});


test('integration checks save settings and explain failures without exposing secrets', async ({ page, request }) => {
  await page.goto('/');
  await expect(page.locator('.deck-grid')).toBeVisible();
  await page.keyboard.press('q');
  await page.getByRole('region', { name: 'Deck editor', exact: true }).getByRole('button', { name: 'Settings', exact: true }).click();
  await page.getByRole('tab', { name: 'Integrations', exact: true }).click();
  await expect(page.getByLabel('OBS connection status')).toHaveText('Not checked');
  await page.getByRole('tab', { name: 'Integrations', exact: true }).click();
  await page.getByLabel('Host', { exact: true }).fill('invalid host');
  await page.getByRole('tab', { name: 'Integrations', exact: true }).click();
  await page.getByLabel('Password', { exact: true }).fill('obs-test-secret');
  await page.getByRole('tab', { name: 'Integrations', exact: true }).click();
  await expect(page.getByLabel('Password', { exact: true })).toHaveAttribute('type', 'password');
  await page.getByRole('button', { name: 'Reveal OBS password' }).click();
  await page.getByRole('tab', { name: 'Integrations', exact: true }).click();
  await expect(page.getByLabel('Password', { exact: true })).toHaveAttribute('type', 'text');
  await page.getByRole('button', { name: 'Hide OBS password' }).click();
  await page.getByRole('tab', { name: 'Integrations', exact: true }).click();
  await page.getByRole('button', { name: 'Save and check OBS connection' }).click();
  await page.getByRole('tab', { name: 'Integrations', exact: true }).click();
  await expect(page.getByLabel('OBS connection status')).toContainText('Connection failed');
  const persisted = await (await request.get('/api/v2/config')).json();
  expect(persisted.config.settings.obs.host).toBe('invalid host');
  const status = await (await request.get('/api/v2/integrations/status')).text();
  expect(status).not.toContain('obs-test-secret');
  await page.getByRole('tab', { name: 'Integrations', exact: true }).click();
  await expect(page.getByLabel('Port', { exact: true })).toHaveAttribute('min', '1');
  await page.getByRole('tab', { name: 'Integrations', exact: true }).click();
  await expect(page.getByLabel('Port', { exact: true })).toHaveAttribute('max', '65535');
});


test('backup inspection and cancellation preserve drafts and applying requires a separate save', async ({ page, request }) => {
  await page.goto('/');
  await expect(page.locator('.deck-grid')).toBeVisible();
  await page.keyboard.press('q');
  await page.getByRole('region', { name: 'Deck editor', exact: true }).getByRole('button', { name: 'Settings', exact: true }).click();
  const backup = structuredClone(initial);
  backup.layout.folders[0].label = 'Restored home';
  backup.extensions = { retained: { custom: true } };
  await page.getByRole('tab', { name: 'Backups', exact: true }).click();
  const input = page.getByLabel('Restore a v2 backup');
  await input.setInputFiles({ name: 'candidate.json', mimeType: 'application/json', buffer: Buffer.from(JSON.stringify(backup)) });
  await expect(page.getByText('candidate.json', { exact: true })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Save changes', exact: true })).toBeDisabled();
  await page.getByRole('button', { name: 'Cancel restore' }).click();
  await expect(page.getByRole('button', { name: 'Apply backup to draft' })).toHaveCount(0);
  await input.setInputFiles({ name: 'invalid.json', mimeType: 'application/json', buffer: Buffer.from('{broken') });
  await expect(page.getByRole('alert')).toContainText('Your draft has not changed');
  await expect(page.getByRole('button', { name: 'Save changes', exact: true })).toBeDisabled();
  await input.setInputFiles({ name: 'candidate.json', mimeType: 'application/json', buffer: Buffer.from(JSON.stringify(backup)) });
  await page.getByRole('button', { name: 'Apply backup to draft' }).click();
  const beforeSave = await (await request.get('/api/v2/config')).json();
  expect(beforeSave.config.layout.folders[0].label).toBe('Home');
  await expect(page.getByRole('button', { name: 'Save changes', exact: true })).toBeEnabled();
  await page.getByRole('button', { name: 'Save changes', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Save changes', exact: true })).toBeDisabled();
  const saved = await (await request.get('/api/v2/config')).json();
  expect(saved.config.layout.folders[0].label).toBe('Restored home');
  expect(saved.config.extensions.retained).toEqual({ custom: true });
});


test('shortcut controls stage typed keys and deletion undo restores a duplicate', async ({ page, request }) => {
  await page.goto('/');
  await expect(page.locator('.deck-grid')).toBeVisible();
  await page.keyboard.press('q');
  await page.getByRole('button', { name: 'Edit Play / pause' }).click();
  await page.getByRole('tab', { name: 'Action', exact: true }).click();
  await page.getByRole('combobox', { name: 'Category', exact: true }).selectOption('input');
  await page.getByRole('tab', { name: 'Action', exact: true }).click();
  await page.getByRole('combobox', { name: 'Type', exact: true }).selectOption('key');
  await page.getByRole('button', { name: 'Use copy shortcut' }).click();
  await expect(page.getByLabel('Key 1', { exact: true })).toHaveValue('ctrl');
  await expect(page.getByLabel('Key 2', { exact: true })).toHaveValue('c');
  await expect(page.getByRole('button', { name: 'Remove key 1', exact: true })).toHaveAccessibleDescription('A shortcut must contain between 1 and 16 keys.');
  await expect(page.getByRole('button', { name: 'Add key', exact: true })).toHaveAccessibleDescription('A shortcut must contain between 1 and 16 keys.');
  await page.getByLabel('Key 2', { exact: true }).fill('v');
  await page.getByRole('button', { name: 'Apply to draft' }).click();
  const pending = await (await request.get('/api/v2/config')).json();
  expect(pending.config.layout.folders[0].buttons[0].action.command.type).toBe('play_pause');
  await page.getByRole('button', { name: 'Edit Play / pause' }).click();
  await page.getByRole('button', { name: 'Duplicate', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Edit Play / pause copy', exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Edit Play / pause copy', exact: true }).click();
  await page.getByRole('button', { name: 'Delete', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Edit Play / pause copy', exact: true })).toHaveCount(0);
  await page.getByRole('button', { name: 'Undo deletion', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Edit Play / pause copy', exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Save changes', exact: true }).click();
  await expect(page.getByLabel('Save status')).toHaveText('All changes saved');
  const saved = await (await request.get('/api/v2/config')).json();
  const buttons = saved.config.layout.folders[0].buttons;
  expect(buttons[0].action.command).toEqual({ type: 'key', keys: ['ctrl', 'v'] });
  const duplicate = buttons.find((button: {label: string}) => button.label === 'Play / pause copy');
  expect(duplicate.id).not.toBe(buttons[0].id);
  expect(duplicate.action.command).toEqual(buttons[0].action.command);
});


test('long translations and missing keys remain usable at 200 percent browser zoom', async ({}, testInfo) => {
  const profile = mkdtempSync(path.join(tmpdir(), 'webdeck-zoom-'));
  mkdirSync(path.join(profile, 'Default'));
  // Chromium stores default zoom by storage partition; x is the default profile partition.
  writeFileSync(path.join(profile, 'Default', 'Preferences'), JSON.stringify({ partition: { default_zoom_level: { x: Math.log(2) / Math.log(1.2) } } }));
  const launch = testInfo.project.use.launchOptions ?? {};
  let context: BrowserContext | undefined;
  try {
  context = await chromium.launchPersistentContext(profile, { ...launch, ...(launch.executablePath ? {} : { channel: 'chromium' }), headless: true, viewport: null, args: [...(launch.args ?? []), '--window-size=720,1000'], baseURL: 'http://127.0.0.1:59996' });
  const page = context.pages()[0] ?? await context.newPage();
  const settingsTitle = 'Ausführliche Einstellungen für dieses gemeinsam verwendete Steuerungsdeck';
  const saveLabel = 'Alle vorgenommenen Änderungen auf diesem Computer speichern';
  await page.route('**/api/v2/translations', async (route) => {
    const response = await route.fetch();
    const body = await response.json();
    Object.assign(body.translations, { lang_code: 'de', ui_settings: settingsTitle, ui_save_changes: saveLabel, ui_edit_named_button: 'Ändern {label}', ui_got_it: 'Diese ausführlichen Hinweise habe ich jetzt verstanden', ui_number_bounds: 'Bitte eine Zahl zwischen {min} und {max} eingeben.' });
    delete body.translations.ui_back_to_deck;
    await route.fulfill({ response, json: body });
  });
  await page.goto('/');
  await expect(page.getByRole('button', { name: 'Play / pause', exact: true })).toBeVisible();
  expect(await page.evaluate(() => devicePixelRatio)).toBe(2);
  expect(await page.evaluate(() => innerWidth)).toBe(360);
  await expect(page.locator('html')).toHaveAttribute('lang', 'de');
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
  await page.getByRole('button', { name: 'Diese ausführlichen Hinweise habe ich jetzt verstanden' }).click();
  await page.keyboard.press('q');
  await page.getByRole('button', { name: 'Ändern Play / pause', exact: true }).click();
  await expect(page.getByRole('dialog')).toBeVisible();
  expect(await page.getByRole('dialog').evaluate((dialog) => { const rect = dialog.getBoundingClientRect(); return rect.top >= 0 && rect.bottom <= window.innerHeight; })).toBe(true);
  await page.keyboard.press('F1');
  await expect(page.locator('dialog[open]')).toHaveCount(1);
  await expect(page.getByRole('heading', { name: 'Edit button', exact: true })).toBeVisible();
  const capture = await context.newCDPSession(page);
  const zoomScreenshot = async (name: string) => { const { data } = await capture.send('Page.captureScreenshot', { format: 'png', fromSurface: true, captureBeyondViewport: false }); writeFileSync(testInfo.outputPath(name), Buffer.from(data, 'base64')); };
  await zoomScreenshot('translated-editor-200-percent.png');
  await page.getByRole('button', { name: 'Apply to draft', exact: true }).scrollIntoViewIfNeeded();
  await expect(page.getByRole('button', { name: 'Apply to draft', exact: true })).toBeInViewport();
  await page.keyboard.press('Escape');
  await expect(page.getByRole('button', { name: 'Ändern Play / pause', exact: true })).toBeFocused();
  await page.keyboard.press('Control+,');
  await expect(page.getByRole('heading', { name: settingsTitle, exact: true })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Back to deck', exact: true })).toBeVisible();
  await page.getByRole('tab', { name: 'Appearance', exact: true }).click();
  await page.getByLabel('Columns', { exact: true }).fill('');
  await expect(page.getByLabel('Columns', { exact: true })).toHaveAttribute('aria-invalid', 'true');
  await expect(page.getByText('Bitte eine Zahl zwischen 1 und 128 eingeben.')).toBeVisible();
  await expect(page.getByRole('button', { name: saveLabel, exact: true })).toBeDisabled();
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
  await zoomScreenshot('translated-settings-200-percent.png');
  } finally { await context?.close(); rmSync(profile, { recursive: true, force: true }); }
});


test('saving Spanish updates controls and action names while preserving configured button content', async ({ page, request }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Settings', exact: true }).click();
  await page.getByRole('combobox', { name: 'Language', exact: true }).selectOption('es_ES');
  await page.getByRole('button', { name: 'Save changes', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Configuración', exact: true })).toBeVisible();
  await expect(page.locator('html')).toHaveAttribute('lang', 'es');
  await page.getByRole('tab', { name: 'Dispositivos', exact: true }).click();
  await expect(page.getByRole('checkbox', { name: 'Ver el panel y las mediciones', exact: true })).toBeVisible();
  const snapshot = await (await request.get('/api/v2/config')).json();
  expect(snapshot.config.settings.language).toBe('es_ES');
  await page.getByRole('button', { name: 'Volver al panel', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Play / pause', exact: true })).toBeVisible();
  await page.keyboard.press('q');
  await page.getByRole('button', { name: 'Editar Play / pause', exact: true }).click();
  await page.getByRole('tab', { name: 'Acción', exact: true }).click();
  await page.getByRole('combobox', { name: 'Categoría', exact: true }).selectOption('input');
  await page.getByRole('combobox', { name: 'Tipo', exact: true }).selectOption('key');
  await expect(page.getByRole('option', { name: 'Atajo de teclado', exact: true })).toHaveAttribute('value', 'key');
  await cancelButtonEditor(page, 'Cancelar');
  const unchanged = await (await request.get('/api/v2/config')).json();
  expect(unchanged.config.layout.folders[0].buttons[0].action.command.type).toBe('play_pause');
});


test('pairing retains failed credentials and meets automated accessibility checks', async ({ page }, testInfo) => {
  await page.addInitScript(() => sessionStorage.setItem('webdeck.device', 'invalid-pairing-token'));
  await page.goto('/');
  await expect(page.getByRole('heading', { name: 'Connect your device', exact: true })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Connect', exact: true })).toBeDisabled();
  await expect(page.getByRole('button', { name: 'Connect', exact: true })).toHaveAccessibleDescription(/On the host computer/);
  await expect(page.getByLabel('Pairing token', { exact: true })).toHaveAccessibleDescription(/one-time pairing token/);
  await captureState(page, testInfo, 'pairing');
  await page.getByLabel('Pairing token', { exact: true }).fill('recoverable-invalid-token');
  await page.getByRole('button', { name: 'Connect', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Connect', exact: true })).toBeEnabled();
  await expect(page.getByLabel('Pairing token', { exact: true })).toHaveValue('recoverable-invalid-token');
  const { default: AxeBuilder } = await import('@axe-core/playwright');
  const review = await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa']).analyze();
  expect(review.violations.map(({ id, nodes }) => ({ id, targets: nodes.map((node) => node.target) }))).toEqual([]);
});

test('native touch hold and release opens controls without executing a command', async ({ browser }, testInfo) => {
  const context = await browser.newContext({ viewport: { width: 360, height: 900 }, isMobile: true, hasTouch: true, baseURL: 'http://127.0.0.1:59996' });
  try {
    const page = await context.newPage();
    let executions = 0;
    page.on('request', (request) => { if (request.method() === 'POST' && request.url().endsWith('/api/v2/commands')) executions++; });
    await page.goto('/');
    const button = page.getByRole('button', { name: 'Play / pause', exact: true });
    await expect(button).toBeVisible();
    await page.screenshot({ path: testInfo.outputPath('touch-deck-360.png'), fullPage: true });
    const rect = await button.boundingBox();
    expect(rect).not.toBeNull();
    const input = await context.newCDPSession(page);
    await input.send('Input.dispatchTouchEvent', { type: 'touchStart', touchPoints: [{ x: rect!.x + rect!.width / 2, y: rect!.y + rect!.height / 2 }] });
    await expect(page.getByRole('dialog', { name: 'Deck controls', exact: true })).toBeVisible();
    await input.send('Input.dispatchTouchEvent', { type: 'touchEnd', touchPoints: [] });
    await page.screenshot({ path: testInfo.outputPath('touch-controls-360.png'), fullPage: true });
    expect(executions).toBe(0);
    await page.getByRole('button', { name: 'Close controls', exact: true }).tap();
    await button.tap();
    await expect(button).toContainText('Completed');
    expect(executions).toBe(1);
  } finally { await context.close(); }
});

test('a delayed token copy cannot clear a newer device approval', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Settings', exact: true }).first().click();
  await page.evaluate(() => {
    Object.defineProperty(navigator, 'clipboard', {
      configurable: true,
      value: { writeText: () => new Promise<void>(resolve => {
        Reflect.set(window, 'finishTokenCopy', resolve);
      }) },
    });
  });
  await page.getByRole('tab', { name: 'Devices', exact: true }).click();
  await page.getByLabel('Device name', { exact: true }).fill('First approval');
  await page.getByRole('button', { name: 'Approve device', exact: true }).click();
  const token = page.locator('#settings-devices .token');
  await expect(token).toBeVisible();
  const first = await token.textContent();
  await page.getByRole('button', { name: 'Copy token', exact: true }).click();
  await expect.poll(() => page.evaluate(() => typeof Reflect.get(window, 'finishTokenCopy'))).toBe('function');
  await page.getByRole('tab', { name: 'Devices', exact: true }).click();
  await page.getByLabel('Device name', { exact: true }).fill('Second approval');
  await page.getByRole('button', { name: 'Approve device', exact: true }).click();
  await expect(token).not.toHaveText(first!);
  const second = await token.textContent();
  await page.evaluate(() => Reflect.get(window, 'finishTokenCopy')());
  await expect(token).toHaveText(second!);
  await page.getByRole('button', { name: 'Hide token', exact: true }).click();
  await expect(token).toHaveCount(0);
});

test('declining reload preserves an unsaved settings draft', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('heading', { name: 'Home', exact: true }).waitFor();
  await page.keyboard.press('Control+,');
  await page.getByRole('tab', { name: 'Appearance', exact: true }).click();
  const columns = page.getByLabel('Columns', { exact: true });
  await columns.fill('5');
  const confirmation = page.waitForEvent('dialog');
  await page.getByRole('tab', { name: 'Backups', exact: true }).click();
  const reload = page.getByRole('button', { name: 'Reload configuration', exact: true }).click();
  const dialog = await confirmation;
  expect(dialog.type()).toBe('confirm');
  await dialog.dismiss();
  await reload;
  await expect(columns).toHaveValue('5');
  await expect(page.getByRole('button', { name: 'Save changes', exact: true })).toBeEnabled();
});

test('browser navigation warns about unsaved work and dismissal preserves the draft', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('heading', { name: 'Home', exact: true }).waitFor();
  await page.keyboard.press('Control+,');
  await page.getByRole('tab', { name: 'Appearance', exact: true }).click();
  const columns = page.getByLabel('Columns', { exact: true });
  await columns.fill('5');
  const warning = page.waitForEvent('dialog');
  const navigation = page.goto('about:blank').catch(() => undefined);
  const dialog = await warning;
  expect(dialog.type()).toBe('beforeunload');
  await dialog.dismiss();
  await navigation;
  await expect(page).toHaveURL(/#settings$/);
  await expect(columns).toHaveValue('5');
  await expect(page.getByRole('button', { name: 'Save changes', exact: true })).toBeEnabled();
});

test('missing uploaded icons show a placeholder instead of an asset identifier', async ({ page, request }) => {
  const snapshot = await (await request.get('/api/v2/config')).json();
  const config = structuredClone(snapshot.config);
  config.layout.folders[0].buttons[0].icon = 'asset:missing-preview.png';
  expect((await request.put('/api/v2/config', { data: { revision: snapshot.revision, config } })).ok()).toBeTruthy();
  await page.goto('/');
  const tile = page.getByRole('button', { name: 'Play / pause', exact: true });
  await expect(tile.locator('.button-icon')).toHaveCount(1);
  await expect(tile).not.toContainText('asset:');
  await expect(page.getByRole('status')).toContainText('Some deck assets are unavailable');
});

test('cancelling span and collision edits preserves the full draft and host revision', async ({ page, request }) => {
  const snapshot = await (await request.get('/api/v2/config')).json();
  const config = structuredClone(snapshot.config);
  config.layout.folders[0].buttons[0].extensions.appearance = { cell: 0, columns: 2, rows: 2 };
  config.layout.folders[0].buttons[1].extensions.appearance = { cell: 2 };
  const stored = await (await request.put('/api/v2/config', { data: { revision: snapshot.revision, config } })).json();
  await page.goto('/');
  const tile = page.getByRole('button', { name: 'Play / pause', exact: true }).locator('..');
  const neighbor = page.locator('.deck-grid').getByRole('button', { name: 'Settings', exact: true }).locator('..');
  await expect(tile).toHaveCSS('grid-column-end', 'span 2');
  await expect(tile).toHaveCSS('grid-row-end', 'span 2');
  await page.keyboard.press('q');
  await page.getByRole('button', { name: 'Edit Play / pause', exact: true }).click();
  await page.getByRole('tab', { name: 'Appearance', exact: true }).click();
  await page.getByLabel('Column span', { exact: true }).fill('3');
  await page.getByLabel('Row span', { exact: true }).fill('3');
  await page.getByRole('button', { name: 'Move right', exact: true }).click();
  await cancelButtonEditor(page);
  await expect(tile).toHaveAttribute('data-cell', '0');
  await expect(tile).toHaveCSS('grid-column-end', 'span 2');
  await expect(tile).toHaveCSS('grid-row-end', 'span 2');
  await expect(neighbor).toHaveAttribute('data-cell', '2');
  await expect(page.getByRole('button', { name: 'Save changes', exact: true })).toBeDisabled();
  const unchanged = await (await request.get('/api/v2/config')).json();
  expect(unchanged.revision).toBe(stored.revision);
  expect(unchanged.config).toEqual(stored.config);
});

test('browser navigation protects unapplied button edits without marking the host draft dirty', async ({ page, request }) => {
  const stored = await (await request.get('/api/v2/config')).json();
  await page.goto('/');
  await page.getByRole('button', { name: 'Play / pause', exact: true }).waitFor();
  await page.keyboard.press('q');
  await page.getByRole('button', { name: 'Edit Play / pause' }).click();
  const label = page.getByLabel('Label', { exact: true });
  await label.fill('Unapplied button edit');
  const warning = page.waitForEvent('dialog');
  const navigation = page.goto('about:blank').catch(() => undefined);
  const dialog = await warning;
  expect(dialog.type()).toBe('beforeunload');
  await dialog.dismiss();
  await navigation;
  await expect(label).toHaveValue('Unapplied button edit');
  expect(await (await request.get('/api/v2/config')).json()).toEqual(stored);
  await cancelButtonEditor(page);
  await expect(page.getByRole('button', { name: 'Save changes', exact: true })).toBeDisabled();
  await page.goto('about:blank');
  await expect(page).toHaveURL('about:blank');
});

test('opening an unchanged button dialog does not warn about draft loss', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Play / pause', exact: true }).waitFor();
  await page.keyboard.press('q');
  await page.getByRole('button', { name: 'Edit Play / pause' }).click();
  await expect(page.getByLabel('Label', { exact: true })).toHaveValue('Play / pause');
  await page.goto('about:blank');
  await expect(page).toHaveURL('about:blank');
});

test('saved notices and command errors remain visible and independently dismissible on mobile', async ({ page }) => {
  await page.setViewportSize({ width: 360, height: 900 });
  await page.goto('/');
  await page.getByRole('button', { name: 'Play / pause', exact: true }).waitFor();
  await page.keyboard.press('q');
  await page.getByRole('button', { name: 'Folder tools', exact: true }).click();
  await page.getByLabel('Folder name', { exact: true }).fill('Notification test');
  await page.getByRole('button', { name: 'Save changes', exact: true }).click();
  await expect(page.locator('.save-status')).toHaveText('All changes saved');
  await page.getByRole('button', { name: 'Done', exact: true }).click();
  await page.route('**/api/v2/commands', route => route.fulfill({ status: 503,
    contentType: 'application/json', body: JSON.stringify({ message: 'Command unavailable' }) }));
  await page.getByRole('button', { name: 'Play / pause', exact: true }).click();
  const error = page.getByRole('alert');
  const notice = page.getByRole('status').filter({ hasText: /^Saved/ });
  await expect(error).toContainText('Command unavailable');
  await expect(notice).toBeVisible();
  const errorBox = await error.boundingBox();
  const noticeBox = await notice.boundingBox();
  expect(errorBox && noticeBox && errorBox.y + errorBox.height <= noticeBox.y).toBeTruthy();
  for (const label of ['Dismiss error', 'Dismiss notice']) {
    const box = await page.getByRole('button', { name: label }).boundingBox();
    expect(box?.width).toBeGreaterThanOrEqual(44);
    expect(box?.height).toBeGreaterThanOrEqual(44);
  }
  await page.getByRole('button', { name: 'Dismiss notice' }).click();
  await expect(error).toBeVisible();
  await expect(notice).toHaveCount(0);
  await page.getByRole('button', { name: 'Dismiss error' }).click();
  await expect(error).toHaveCount(0);
});

test('Spotify continuation links cannot outlive credential edits during authorization', async ({ page, request }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Play / pause', exact: true }).waitFor();
  await page.keyboard.press('Control+,');
  await page.getByRole('tab', { name: 'Integrations', exact: true }).click();
  await page.getByLabel('Client ID', { exact: true }).fill('first-client');
  await page.getByRole('tab', { name: 'Integrations', exact: true }).click();
  await page.getByLabel('Client secret', { exact: true }).fill('first-secret');
  let release!: () => void;
  let observed!: () => void;
  const pending = new Promise<void>(resolve => { observed = resolve; });
  let attempts = 0;
  await page.route('**/api/v2/spotify/connect', async route => {
    const attempt = ++attempts;
    if (attempt === 1) {
      const barrier = new Promise<void>(resolve => { release = resolve; });
      observed();
      await barrier;
    }
    await route.fulfill({ contentType: 'application/json', body: JSON.stringify({
      api_version: 2, url: `https://accounts.spotify.com/authorize?state=attempt-${attempt}`,
    }) });
  });
  await page.getByRole('tab', { name: 'Integrations', exact: true }).click();
  await page.getByRole('button', { name: 'Connect Spotify', exact: true }).click();
  await pending;
  await expect(page.getByRole('button', { name: 'Save and check OBS connection', exact: true })).toBeDisabled();
  await expect(page.getByRole('button', { name: 'Save and check OBS connection', exact: true })).toHaveAccessibleDescription('Preparing authorization…');
  await page.getByRole('tab', { name: 'Integrations', exact: true }).click();
  await page.getByLabel('Client secret', { exact: true }).fill('new-secret');
  release();
  await page.getByRole('tab', { name: 'Integrations', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Connect Spotify', exact: true })).toBeEnabled();
  const continuation = page.getByRole('link', { name: 'Continue to Spotify', exact: true });
  await expect(continuation).toHaveCount(0);
  expect((await (await request.get('/api/v2/config')).json()).config.settings.spotify.client_secret).toBe('first-secret');
  await page.getByRole('tab', { name: 'Integrations', exact: true }).click();
  await expect(page.getByLabel('Client secret', { exact: true })).toHaveValue('new-secret');
  await page.getByRole('tab', { name: 'Integrations', exact: true }).click();
  await page.getByRole('button', { name: 'Connect Spotify', exact: true }).click();
  await expect(continuation).toHaveAttribute('href', /state=attempt-2$/);
  expect((await (await request.get('/api/v2/config')).json()).config.settings.spotify.client_secret).toBe('new-secret');
  await page.getByRole('tab', { name: 'Integrations', exact: true }).click();
  await page.getByLabel('Client ID', { exact: true }).fill('third-client');
  await expect(continuation).toHaveCount(0);
  expect(attempts).toBe(2);
});

test('a late initial integration snapshot cannot replace an explicit OBS check', async ({ page }) => {
  let release!: () => void;
  let observed!: () => void;
  let completed!: () => void;
  const pending = new Promise<void>(resolve => { observed = resolve; });
  const finished = new Promise<void>(resolve => { completed = resolve; });
  await page.route('**/api/v2/integrations/status', async route => {
    const barrier = new Promise<void>(resolve => { release = resolve; });
    observed();
    await barrier;
    await route.fulfill({ contentType: 'application/json', body: JSON.stringify({
      api_version: 2, obs: 'not_tested', spotify: 'authorization_required', checked_at: 0,
    }) });
    completed();
  });
  await page.route('**/api/v2/integrations/obs/check', route => route.fulfill({
    contentType: 'application/json', body: JSON.stringify({
      api_version: 2, obs: 'failed', spotify: 'authorization_required', checked_at: 1,
    }),
  }));
  await page.goto('/');
  await page.getByRole('button', { name: 'Play / pause', exact: true }).waitFor();
  await page.keyboard.press('Control+,');
  await pending;
  await page.getByRole('tab', { name: 'Integrations', exact: true }).click();
  await page.getByRole('button', { name: 'Save and check OBS connection', exact: true }).click();
  await page.getByRole('tab', { name: 'Integrations', exact: true }).click();
  await expect(page.getByLabel('OBS connection status')).toContainText('Connection failed');
  const response = page.waitForResponse('**/api/v2/integrations/status');
  release();
  await finished;
  await (await response).finished();
  // Drain the browser's response handling before inspecting the displayed result.
  await page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
  await page.getByRole('tab', { name: 'Integrations', exact: true }).click();
  await expect(page.getByLabel('OBS connection status')).toContainText('Connection failed');
});

test('keyboard-only editing saves a staged button with reduced motion and restored focus', async ({ page, request }) => {
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto('/');
  const play = page.getByRole('button', { name: 'Play / pause', exact: true });
  await play.waitFor();
  let commands = 0;
  page.on('request', request => {
    if (request.method() === 'POST' && request.url().endsWith('/api/v2/commands')) commands++;
  });
  const tabTo = async (target: ReturnType<Page['getByRole']>) => {
    for (let step = 0; step < 60; step++) {
      if (await target.evaluate(element => element === document.activeElement)) return;
      await page.keyboard.press('Tab');
    }
    throw new Error('Keyboard target was unreachable');
  };
  await tabTo(play);
  await page.keyboard.press('F1');
  const help = page.getByRole('dialog', { name: 'Keyboard shortcuts', exact: true });
  await expect(help).toBeVisible();
  await page.keyboard.press('Shift+Tab');
  expect(await help.evaluate(element => element.contains(document.activeElement))).toBe(true);
  await page.keyboard.press('Escape');
  await expect(play).toBeFocused();
  await page.keyboard.press('q');
  const edit = page.getByRole('button', { name: 'Edit Play / pause', exact: true });
  await tabTo(edit);
  await expect(edit).toHaveCSS('outline-style', 'solid');
  await page.keyboard.down('Space');
  await expect(edit).toHaveCSS('transform', 'none');
  await page.keyboard.up('Space');
  const dialog = page.getByRole('dialog', { name: 'Edit button', exact: true });
  await expect(dialog).toBeVisible();
  const label = page.getByLabel('Label', { exact: true });
  await tabTo(label);
  await page.keyboard.press('Control+a');
  await page.keyboard.type('Keyboard music');
  await tabTo(page.getByRole('button', { name: 'Apply to draft', exact: true }));
  await page.keyboard.press('Enter');
  await expect(dialog).toHaveCount(0);
  await expect(page.getByRole('button', { name: 'Edit Keyboard music', exact: true })).toBeFocused();
  const save = page.getByRole('button', { name: 'Save changes', exact: true });
  await tabTo(save);
  await page.keyboard.press('Enter');
  await expect(page.locator('.save-status')).toHaveText('All changes saved');
  await tabTo(page.getByRole('button', { name: 'Done', exact: true }));
  await page.keyboard.press('Enter');
  await expect(page.locator('.deck-grid')).not.toHaveClass(/editing/);
  const stored = await (await request.get('/api/v2/config')).json();
  expect(stored.config.layout.folders[0].buttons[0].label).toBe('Keyboard music');
  expect(commands).toBe(0);
});

test('folder deletion undo restores incoming links, contents, and the complete saved configuration', async ({ page, request }) => {
  const original = await (await request.get('/api/v2/config')).json();
  await page.goto('/');
  await page.getByRole('button', { name: 'Work folder', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Work', exact: true })).toBeVisible();
  await page.keyboard.press('q');
  await page.getByRole('button', { name: 'Delete folder', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Work folder', exact: true })).toHaveCount(0);
  await expect(page.getByRole('button', { name: 'Undo deletion', exact: true })).toBeVisible();
  expect(await (await request.get('/api/v2/config')).json()).toEqual(original);
  await page.getByRole('button', { name: 'Undo deletion', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Work folder', exact: true })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Save changes', exact: true })).toBeDisabled();
  await expect(page.locator('.save-status')).toHaveText('All changes saved');
  expect(await (await request.get('/api/v2/config')).json()).toEqual(original);
});

test('declining backup replacement retains the full unsaved draft and candidate for deliberate retry', async ({ page, request }) => {
  const original = await (await request.get('/api/v2/config')).json();
  await page.goto('/');
  await page.getByRole('button', { name: 'Play / pause', exact: true }).waitFor();
  await page.keyboard.press('Control+,');
  await page.getByRole('tab', { name: 'Appearance', exact: true }).click();
  await page.getByLabel('Columns', { exact: true }).fill('5');
  const backup = structuredClone(original.config);
  backup.layout.columns = 7;
  backup.extensions = { restored: { retained: true } };
  await page.getByRole('tab', { name: 'Backups', exact: true }).click();
  await page.getByLabel('Restore a v2 backup', { exact: true }).setInputFiles({
    name: 'replacement.json', mimeType: 'application/json', buffer: Buffer.from(JSON.stringify(backup)),
  });
  await expect(page.getByText('replacement.json', { exact: true })).toBeVisible();
  const warning = page.waitForEvent('dialog');
  const applying = page.getByRole('button', { name: 'Apply backup to draft', exact: true }).click();
  const dialog = await warning;
  expect(dialog.type()).toBe('confirm');
  expect(dialog.message()).toContain('unsaved');
  await dialog.dismiss();
  await applying;
  await expect(page.getByLabel('Columns', { exact: true })).toHaveValue('5');
  await expect(page.getByRole('button', { name: 'Apply backup to draft', exact: true })).toBeVisible();
  expect(await (await request.get('/api/v2/config')).json()).toEqual(original);
  const downloadEvent = page.waitForEvent('download');
  await page.getByRole('tab', { name: 'Backups', exact: true }).click();
  await page.getByRole('button', { name: 'Download backup', exact: true }).click();
  const downloaded = await (await downloadEvent).path();
  expect(downloaded).not.toBeNull();
  const preserved = structuredClone(original.config);
  preserved.layout.columns = 5;
  expect(JSON.parse(readFileSync(downloaded!, 'utf8'))).toEqual(preserved);
  const accepted = page.waitForEvent('dialog');
  const retry = page.getByRole('button', { name: 'Apply backup to draft', exact: true }).click();
  await (await accepted).accept();
  await retry;
  await expect(page.getByLabel('Columns', { exact: true })).toHaveValue('7');
  expect(await (await request.get('/api/v2/config')).json()).toEqual(original);
  await page.getByRole('button', { name: 'Save changes', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Save changes', exact: true })).toBeDisabled();
  expect((await (await request.get('/api/v2/config')).json()).config).toEqual(backup);
});

test('device loading failures stay distinct from an empty list and support retry', async ({ page }) => {
  // Approval guidance is checked independently of the list request's outcome below.
  let release!: () => void;
  let observed!: () => void;
  const pending = new Promise<void>(resolve => { observed = resolve; });
  let attempts = 0;
  await page.route('**/api/v2/devices', async route => {
    expect(route.request().method()).toBe('GET');
    if (++attempts === 1) {
      const barrier = new Promise<void>(resolve => { release = resolve; });
      observed();
      await barrier;
      await route.fulfill({ status: 503, contentType: 'application/json',
        body: JSON.stringify({ message: 'Device list unavailable' }) });
    } else {
      await route.fulfill({ contentType: 'application/json',
        body: JSON.stringify({ api_version: 2, devices: [] }) });
    }
  });
  await page.goto('/');
  await page.getByRole('button', { name: 'Play / pause', exact: true }).waitFor();
  await page.keyboard.press('Control+,');
  await pending;
  await page.getByRole('tab', { name: 'Devices', exact: true }).click();
  const devices = page.locator('#settings-devices');
  await expect(devices.getByRole('button', { name: 'Approve device' })).toBeDisabled();
  await expect(devices.getByRole('button', { name: 'Approve device' })).toHaveAccessibleDescription('Enter a device name and select at least one permission to approve it.');
  await expect(devices.getByRole('status')).toHaveText('Loading devices…');
  await expect(devices.getByText('No devices paired yet.', { exact: true })).toHaveCount(0);
  release();
  await expect(devices.getByRole('alert')).toHaveText('Device list unavailable');
  await expect(devices.getByRole('status')).toHaveCount(0);
  await expect(devices.getByText('No devices paired yet.', { exact: true })).toHaveCount(0);
  await devices.getByRole('button', { name: 'Retry', exact: true }).click();
  await expect(devices.getByRole('alert')).toHaveCount(0);
  await expect(devices.getByText('No devices paired yet.', { exact: true })).toBeVisible();
  expect(attempts).toBe(2);
});


test('a failed subsequent save clears the previous saved notice and preserves the new draft', async ({ page, request }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Play / pause', exact: true }).waitFor();
  await page.keyboard.press('q');
  await page.getByLabel('Folder name', { exact: true }).fill('Saved folder');
  await page.getByRole('button', { name: 'Save changes', exact: true }).click();
  await expect(page.getByRole('status').filter({ hasText: /^Saved/ })).toBeVisible();
  await page.getByLabel('Folder name', { exact: true }).fill('New unsaved folder');
  const latest = await (await request.get('/api/v2/config')).json();
  latest.config.layout.rows = 4;
  expect((await request.put('/api/v2/config', { data: { revision: latest.revision, config: latest.config } })).ok()).toBeTruthy();
  await page.getByRole('button', { name: 'Save changes', exact: true }).click();
  await expect(page.getByRole('alert')).toContainText('draft');
  await expect(page.getByRole('status').filter({ hasText: /^Saved/ })).toHaveCount(0);
  await expect(page.getByLabel('Folder name', { exact: true })).toHaveValue('New unsaved folder');
});


test('reverting a folder edit removes the unsaved warning without writing to the host', async ({ page, request }) => {
  const original = await (await request.get('/api/v2/config')).json();
  await page.goto('/');
  await page.getByRole('button', { name: 'Play / pause', exact: true }).waitFor();
  await page.keyboard.press('q');
  const field = page.getByLabel('Folder name', { exact: true });
  await field.fill('Temporary edit');
  await expect(page.locator('.save-status')).toHaveText('Unsaved changes');
  await field.fill('Home');
  await expect(page.locator('.save-status')).toHaveText('All changes saved');
  await expect(page.getByRole('button', { name: 'Save changes', exact: true })).toBeDisabled();
  const dialogs: string[] = [];
  page.on('dialog', async dialog => { dialogs.push(dialog.type()); await dialog.dismiss(); });
  await page.reload();
  await page.getByRole('button', { name: 'Play / pause', exact: true }).waitFor();
  expect(dialogs).toEqual([]);
  expect(await (await request.get('/api/v2/config')).json()).toEqual(original);
});


test('distant sparse rows stay bounded and retain button coordinates through editing', async ({ page, request }, testInfo) => {
  const snapshot = await (await request.get('/api/v2/config')).json();
  snapshot.config.layout.folders[0].buttons[0].extensions.appearance = { cell: 2 ** 40, rows: 3, columns: 2 };
  await request.put('/api/v2/config', { data: { revision: snapshot.revision, config: snapshot.config } });
  const original = await (await request.get('/api/v2/config')).json();
  const errors: string[] = [];
  page.on('pageerror', error => errors.push(error.message));
  await page.goto('/');
  await page.getByLabel('First visible row', { exact: true }).fill(String(2 ** 38 + 1));
  await expect(page.getByRole('button', { name: 'Play / pause', exact: true })).toBeVisible();
  expect(await page.locator('.deck-cell').count()).toBeLessThanOrEqual(512);
  await expect(page.locator(`[data-cell="${2 ** 40}"]`)).toHaveCSS('grid-row-start', '1');
  await page.getByLabel('First visible row', { exact: true }).fill(String(2 ** 38 + 2));
  await expect(page.getByRole('button', { name: 'Play / pause', exact: true })).toBeVisible();
  await expect(page.locator(`[data-cell="${2 ** 40}"]`)).toHaveCSS('grid-row-start', '1');
  await expect(page.locator(`[data-cell="${2 ** 40}"]`)).toHaveCSS('grid-row-end', 'span 2');
  await page.setViewportSize({ width: 360, height: 900 });
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
  const { default: AxeBuilder } = await import('@axe-core/playwright');
  const accessibility = await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa']).analyze();
  expect(accessibility.violations.map(violation => violation.id)).toEqual([]);
  await page.screenshot({ path: testInfo.outputPath('sparse-crossing-360.png'), fullPage: true });
  await expect(page.getByRole('button', { name: 'Next rows', exact: true })).toBeDisabled();
  await page.getByRole('button', { name: 'Previous rows', exact: true }).click();
  await expect(page.getByLabel('First visible row', { exact: true })).toHaveValue(String(2 ** 38 + 2 - 128));
  await page.getByRole('button', { name: 'Next rows', exact: true }).click();
  await expect(page.getByLabel('First visible row', { exact: true })).toHaveValue(String(2 ** 38 + 2));
  await page.getByRole('region', { name: 'Control deck', exact: true }).focus();
  await page.keyboard.press('q');
  await page.getByRole('button', { name: 'Edit Play / pause', exact: true }).click();
  await cancelButtonEditor(page);
  expect(await (await request.get('/api/v2/config')).json()).toEqual(original);
  await page.getByRole('region', { name: 'Control deck', exact: true }).focus();
  await page.keyboard.press('q');
  await page.getByLabel('First visible row', { exact: true }).fill('1');
  await page.getByRole('button', { name: 'Work folder', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Work', exact: true })).toBeVisible();
  await expect(page.getByLabel('First visible row', { exact: true })).toHaveCount(0);
  await page.keyboard.press('Alt+ArrowLeft');
  await expect(page.getByLabel('First visible row', { exact: true })).toHaveValue('1');
  expect(errors).toEqual([]);
});


test('unsupported numeric placements show an error without changing the host or breaking settings', async ({ page, request }) => {
  const snapshot = await (await request.get('/api/v2/config')).json();
  snapshot.config.layout.folders[0].buttons[0].extensions.appearance = { cell: Number.MAX_SAFE_INTEGER, rows: 2 };
  expect((await request.put('/api/v2/config', { data: { revision: snapshot.revision, config: snapshot.config } })).ok()).toBeTruthy();
  const original = await (await request.get('/api/v2/config')).json();
  const errors: string[] = [];
  page.on('pageerror', error => errors.push(error.message));
  await page.goto('/');
  await expect(page.getByRole('alert')).toHaveText('Grid position exceeds the supported integer range.');
  await page.keyboard.press('Control+,');
  await expect(page.getByRole('heading', { name: 'Settings', exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Back to deck', exact: true }).click();
  await expect(page.getByRole('alert')).toHaveText('Grid position exceeds the supported integer range.');
  expect(await (await request.get('/api/v2/config')).json()).toEqual(original);
  expect(errors).toEqual([]);
});


test('the final supported cell remains visible in a partial row without changing its placement', async ({ page, request }) => {
  const snapshot = await (await request.get('/api/v2/config')).json();
  snapshot.config.layout.columns = 3;
  snapshot.config.layout.folders[0].buttons[0].extensions.appearance = { cell: Number.MAX_SAFE_INTEGER };
  expect((await request.put('/api/v2/config', { data: { revision: snapshot.revision, config: snapshot.config } })).ok()).toBeTruthy();
  const original = await (await request.get('/api/v2/config')).json();
  const errors: string[] = [];
  page.on('pageerror', error => errors.push(error.message));
  await page.goto('/');
  await page.getByLabel('First visible row', { exact: true }).fill(String(Math.floor(Number.MAX_SAFE_INTEGER / 3) + 1));
  await expect(page.getByRole('button', { name: 'Play / pause', exact: true })).toBeVisible();
  await expect(page.locator(`[data-cell="${Number.MAX_SAFE_INTEGER}"]`)).toHaveCSS('grid-column-start', '2');
  await expect(page.getByRole('button', { name: 'Next rows', exact: true })).toBeDisabled();
  expect(await page.locator('.deck-cell').count()).toBe(2);
  await page.getByRole('region', { name: 'Control deck', exact: true }).focus();
  await page.keyboard.press('q');
  await page.getByRole('button', { name: 'Edit Play / pause', exact: true }).click();
  await cancelButtonEditor(page);
  expect(await (await request.get('/api/v2/config')).json()).toEqual(original);
  expect(errors).toEqual([]);
});


test('touch dialog controls retain a 44 pixel target in both dimensions', async ({ browser, request }) => {
  const snapshot = await (await request.get('/api/v2/config')).json();
  snapshot.config.layout.columns = 8;
  expect((await request.put('/api/v2/config', { data: { revision: snapshot.revision, config: snapshot.config } })).ok()).toBeTruthy();
  const original = await (await request.get('/api/v2/config')).json();
  const context = await browser.newContext({ viewport: { width: 360, height: 900 }, isMobile: true, hasTouch: true });
  try {
    const page = await context.newPage();
    await page.goto('http://127.0.0.1:59996/');
    await page.getByRole('button', { name: 'Play / pause', exact: true }).waitFor();
    await page.keyboard.press('q');
    await page.setViewportSize({ width: 768, height: 900 });
    for (const name of ['Edit Play / pause']) {
      const bounds = await page.getByRole('button', { name, exact: true }).boundingBox();
      expect(bounds?.width).toBeGreaterThanOrEqual(44);
      expect(bounds?.height).toBeGreaterThanOrEqual(44);
    }
    await page.setViewportSize({ width: 360, height: 900 });
    await page.getByRole('button', { name: 'Edit Play / pause', exact: true }).click();
    const dialog = page.getByRole('dialog', { name: 'Edit button', exact: true });
    await expect(dialog).toBeVisible();
    for (const name of ['Close editor', 'Apply to draft', 'Cancel', 'Delete']) {
      const bounds = await dialog.getByRole('button', { name, exact: true }).boundingBox();
      expect(bounds?.width).toBeGreaterThanOrEqual(44);
      expect(bounds?.height).toBeGreaterThanOrEqual(44);
    }
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
    expect(await (await request.get('/api/v2/config')).json()).toEqual(original);
  } finally { await context.close(); }
});


test('metric failures distinguish missing and stale readings and recover on the next poll', async ({ page }) => {
  let available = false;
  let cpuPercent = 42;
  await page.route('**/api/v2/usage', async route => {
    if (!available) { await route.fulfill({ status: 503, json: { error: 'usage unavailable' } }); return; }
    await route.fulfill({ json: { api_version: 2, usage: { cpu_percent: cpuPercent, cpus: [], memory_used: 1, memory_total: 2, disks: [], gpus: [] } } });
  });
  await page.goto('/');
  const cpu = page.getByRole('button', { name: 'CPU', exact: true });
  await expect(cpu.locator('.metric-value')).toHaveText('—');
  await expect(cpu.locator('small')).toHaveText('Reading unavailable / stale');
  await expect(cpu.getByRole('progressbar')).toHaveCount(0);
  available = true;
  await expect(cpu.locator('.metric-value')).toHaveText('42.0%');
  await expect(cpu.locator('small')).toHaveText('');
  await expect(cpu.getByRole('progressbar')).toHaveAttribute('value', '42');
  available = false;
  await expect(cpu.locator('small')).toHaveText('Reading unavailable / stale');
  await expect(cpu.locator('.metric-value')).toHaveText('42.0%');
  available = true;
  cpuPercent = 63;
  await expect(cpu.locator('.metric-value')).toHaveText('63.0%');
  await expect(cpu.locator('small')).toHaveText('');
});


test('running commands explain the disabled tile and never repeat the request', async ({ page }) => {
  let release!: () => void;
  const held = new Promise<void>(resolve => { release = resolve; });
  let commandRequests = 0;
  await page.route('**/api/v2/commands', async route => {
    if (route.request().method() !== 'POST') { await route.continue(); return; }
    commandRequests++;
    await held;
    await route.fulfill({ status: 503, json: { message: 'Command unavailable' } });
  });
  try {
    await page.goto('/');
    const play = page.getByRole('button', { name: 'Play / pause', exact: true });
    await play.click();
    await expect(play).toBeDisabled();
    await expect(play).toHaveAccessibleDescription('Running…');
    await expect(play.locator('small')).toHaveText('Running…');
    expect(commandRequests).toBe(1);
    release();
    await expect(play).toBeEnabled();
    await expect(play).toHaveAccessibleDescription(/Command unavailable/);
    await expect(play.locator('small')).toHaveText('Failed');
    expect(commandRequests).toBe(1);
  } finally { release(); }
});

test('dismissed first-use tips stay dismissed after reload without adding deck chrome', async ({ page, request }) => {
  const original = await (await request.get('/api/v2/config')).json();
  await page.goto('/');
  const hint = page.getByRole('note', { name: 'Deck tips', exact: true });
  await expect(hint).toBeVisible();
  await hint.getByRole('button', { name: 'Got it', exact: true }).click();
  await expect(hint).toHaveCount(0);
  await page.reload();
  await expect(page.getByRole('button', { name: 'Play / pause', exact: true })).toBeVisible();
  await expect(hint).toHaveCount(0);
  await expect(page.locator('header, aside, footer')).toHaveCount(0);
  expect(await (await request.get('/api/v2/config')).json()).toEqual(original);
});


test('toolbar button creation stages a free cell and categories show readable labels', async ({ page, request }) => {
  const original = await (await request.get('/api/v2/config')).json();
  await page.goto('/');
  await page.getByRole('button', { name: 'Play / pause', exact: true }).waitFor();
  await page.keyboard.press('q');
  const toolbar = page.getByRole('region', { name: 'Deck editor', exact: true });
  await toolbar.getByRole('button', { name: 'Add button', exact: true }).click();
  await cancelButtonEditor(page);
  await expect(page.getByRole('button', { name: 'Save changes', exact: true })).toBeDisabled();
  await toolbar.getByRole('button', { name: 'Add button', exact: true }).click();
  await page.getByRole('tab', { name: 'Action', exact: true }).click();
  const category = page.getByRole('combobox', { name: 'Category', exact: true });
  await expect(category.locator('option[value="input"]')).toHaveText('Keyboard and text');
  await category.selectOption('input');
  await page.getByRole('tab', { name: 'Content', exact: true }).click();
  await page.getByLabel('Label', { exact: true }).fill('Toolbar button');
  await page.getByRole('button', { name: 'Apply to draft', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Edit Toolbar button', exact: true })).toBeVisible();
  expect(await (await request.get('/api/v2/config')).json()).toEqual(original);
  await page.getByRole('button', { name: 'Save changes', exact: true }).click();
  await expect(page.locator('.save-status')).toHaveText('All changes saved');
  const saved = await (await request.get('/api/v2/config')).json();
  const before = original.config.layout.folders[0].buttons;
  const after = saved.config.layout.folders[0].buttons;
  expect(after.map((button: {id: string}) => button.id).slice(0, before.length)).toEqual(before.map((button: {id: string}) => button.id));
  expect(after.at(-1).label).toBe('Toolbar button');
  expect(after.at(-1).extensions.appearance.cell).toBe(before.length);
});


test('invalid JSON blocks Apply and exposes a field description until corrected', async ({ page, request }) => {
  const original = await (await request.get('/api/v2/config')).json();
  await page.goto('/');
  await page.getByRole('button', { name: 'Play / pause', exact: true }).waitFor();
  await page.keyboard.press('q');
  await page.getByRole('button', { name: 'Edit Play / pause', exact: true }).click();
  await page.getByRole('tab', { name: 'Action', exact: true }).click();
  await page.getByRole('combobox', { name: 'Type', exact: true }).selectOption('debug');
  await page.getByRole('button', { name: 'Add field', exact: true }).click();
  const data = page.getByLabel('field1', { exact: true });
  await data.fill('{broken');
  await data.blur();
  await expect(data).toHaveAttribute('aria-invalid', 'true');
  await expect(data).toHaveAccessibleDescription(/Enter valid JSON/);
  await page.getByRole('button', { name: 'Apply to draft', exact: true }).click();
  await expect(page.getByRole('dialog', { name: 'Edit button', exact: true })).toBeVisible();
  expect(await (await request.get('/api/v2/config')).json()).toEqual(original);
  await data.fill('{"retained":42}');
  await data.blur();
  await expect(data).toHaveAttribute('aria-invalid', 'false');
  await page.getByRole('button', { name: 'Apply to draft', exact: true }).click();
  await expect(page.getByRole('dialog', { name: 'Edit button', exact: true })).toHaveCount(0);
  await page.getByRole('button', { name: 'Save changes', exact: true }).click();
  await expect(page.locator('.save-status')).toHaveText('All changes saved');
  const saved = await (await request.get('/api/v2/config')).json();
  expect(saved.config.layout.folders[0].buttons[0].action.command).toEqual({ type: 'debug', data: { field1: { retained: 42 } } });
});


test('live preview and numeric validation preserve the host until a valid draft is saved', async ({ page, request }) => {
  const original = await (await request.get('/api/v2/config')).json();
  await page.goto('/');
  await page.getByRole('button', { name: 'Play / pause', exact: true }).waitFor();
  await page.keyboard.press('q');
  await page.getByRole('button', { name: 'Edit Play / pause', exact: true }).click();
  await page.getByLabel('Label', { exact: true }).fill('Preview label');
  await page.getByLabel('Color', { exact: true }).fill('#ffffff');
  const preview = page.getByRole('img', { name: 'Button preview', exact: true });
  await expect(preview).toContainText('Preview label');
  await expect(preview).toHaveCSS('color', 'rgb(0, 0, 0)');
  await page.getByRole('tab', { name: 'Appearance', exact: true }).click();
  const span = page.getByLabel('Column span', { exact: true });
  await span.fill('');
  await expect(span).toHaveAttribute('aria-invalid', 'true');
  await expect(span).toHaveAccessibleDescription('Enter a number from 1 to 4.');
  await page.getByRole('button', { name: 'Apply to draft', exact: true }).click();
  await expect(page.getByRole('dialog', { name: 'Edit button', exact: true })).toBeVisible();
  expect(await (await request.get('/api/v2/config')).json()).toEqual(original);
  await span.fill('2');
  await expect(span).toHaveAttribute('aria-invalid', 'false');
  await page.getByRole('button', { name: 'Apply to draft', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Edit Preview label', exact: true })).toBeVisible();
  expect(await (await request.get('/api/v2/config')).json()).toEqual(original);
  await page.getByRole('button', { name: 'Save changes', exact: true }).click();
  await expect(page.locator('.save-status')).toHaveText('All changes saved');
  const saved = await (await request.get('/api/v2/config')).json();
  expect(saved.config.layout.folders[0].buttons[0].extensions.appearance.columns).toBe(2);
  expect(saved.config.layout.folders[0].buttons[0].label).toBe('Preview label');
});


test('pattern validation blocks a malformed reference and accepts a corrected stable ID', async ({ page, request }) => {
  const original = await (await request.get('/api/v2/config')).json();
  const targetId = original.config.layout.folders[1].buttons.find((button: { action: { type: string } }) => button.action.type === 'command').id;
  await page.goto('/');
  await page.getByRole('button', { name: 'Play / pause', exact: true }).waitFor();
  await page.keyboard.press('q');
  await page.getByRole('button', { name: 'Edit Play / pause', exact: true }).click();
  await page.getByRole('tab', { name: 'Action', exact: true }).click();
  await page.getByRole('combobox', { name: 'Type', exact: true }).selectOption('button');
  const reference = page.getByLabel('button id', { exact: true });
  await reference.fill('../invalid');
  await expect(reference).toHaveAttribute('aria-invalid', 'true');
  await expect(reference).toHaveAccessibleDescription(/enter a valid value/);
  await page.getByRole('button', { name: 'Apply to draft', exact: true }).click();
  await expect(page.getByRole('dialog', { name: 'Edit button', exact: true })).toBeVisible();
  expect(await (await request.get('/api/v2/config')).json()).toEqual(original);
  await reference.fill(targetId);
  await expect(reference).toHaveAttribute('aria-invalid', 'false');
  await page.getByRole('button', { name: 'Apply to draft', exact: true }).click();
  await expect(page.getByRole('dialog', { name: 'Edit button', exact: true })).toHaveCount(0);
  await page.getByRole('button', { name: 'Save changes', exact: true }).click();
  await expect(page.locator('.save-status')).toHaveText('All changes saved');
  const saved = await (await request.get('/api/v2/config')).json();
  expect(saved.config.layout.folders[0].buttons[0].action.command).toEqual({ type: 'button', button_id: targetId });
});


test('mobile settings navigation reaches its sections and keeps transport in advanced connection', async ({ page }) => {
  await page.setViewportSize({ width: 360, height: 900 });
  await page.goto('/');
  await page.getByRole('button', { name: 'Play / pause', exact: true }).waitFor();
  await page.keyboard.press('Control+,');
  const navigation = page.getByRole('tablist', { name: 'Settings sections', exact: true });
  await expect(navigation.getByRole('tab')).toHaveCount(6);
  await navigation.getByRole('tab', { name: 'Backups', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Backups', exact: true })).toBeInViewport();
  await navigation.getByRole('tab', { name: 'Connection', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Advanced connection', exact: true })).toBeInViewport();
  await expect(page.locator('#settings-connection').getByRole('combobox', { name: 'Connection', exact: true })).toBeVisible();
  await expect(page.locator('#settings-appearance').getByRole('combobox', { name: 'Connection', exact: true })).toHaveCount(0);
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
});


test('pending device revocation explains its disabled state and becomes visibly revoked', async ({ page, request }) => {
  expect((await request.post('/api/v2/devices', { data: { name: 'Revoke description', capabilities: ['read'], ttl_seconds: 3600 } })).ok()).toBeTruthy();
  await page.goto('/');
  await page.getByRole('button', { name: 'Play / pause', exact: true }).waitFor();
  await page.keyboard.press('Control+,');
  await page.getByRole('tab', { name: 'Devices', exact: true }).click();
  const row = page.locator('#settings-devices .row').filter({ hasText: 'Revoke description' });
  const revoke = row.getByRole('button', { name: 'Revoke', exact: true });
  let release!: () => void;
  const held = new Promise<void>(resolve => { release = resolve; });
  await page.route('**/api/v2/devices/*', async route => {
    if (route.request().method() === 'DELETE') await held;
    await route.continue();
  });
  try {
    await revoke.click();
    await expect(revoke).toBeDisabled();
    await expect(revoke).toHaveAccessibleDescription('Revoking device…');
    release();
    await expect(revoke).toHaveAccessibleDescription('Revoked');
    await expect(revoke).toBeDisabled();
  } finally { release(); }
});

test('missing metric readings stay readable on narrow tiles', async ({ page }, testInfo) => {
  await page.setViewportSize({ width: 360, height: 900 });
  await page.goto('/');
  const gpu = page.getByRole('button', { name: 'GPU', exact: true });
  const reading = gpu.locator('.metric-value');
  await expect(reading).toHaveText('No data');
  const bounds = await reading.evaluate((element) => {
    const range = document.createRange();
    range.selectNodeContents(element);
    const lines = Array.from(range.getClientRects());
    return { lines: lines.length, within: lines.every(rect => {
      const tile = element.closest('.deck-button')!.getBoundingClientRect();
      return rect.left >= tile.left && rect.right <= tile.right;
    }) };
  });
  expect(bounds.lines).toBe(1);
  expect(bounds.within).toBe(true);
  await page.screenshot({ path: testInfo.outputPath('missing-metric-360.png') });
});

test('touch settings controls expose usable targets including checkbox labels and navigation', async ({ browser }) => {
  const context = await browser.newContext({ viewport: { width: 360, height: 900 }, isMobile: true, hasTouch: true });
  try {
    const page = await context.newPage();
    await page.goto('http://127.0.0.1:59996/');
    await page.getByRole('button', { name: 'Play / pause', exact: true }).waitFor();
    await page.keyboard.press('Control+,');
    await expect(page.getByRole('heading', { name: 'Settings', exact: true })).toBeVisible();
    const targets = [];
    for (const tab of ['Appearance', 'Integrations', 'Devices', 'Backups', 'Runtime and plugins', 'Connection']) {
    await page.getByRole('tab', { name: tab, exact: true }).click();
    targets.push(...await page.locator('main').evaluate(main => Array.from(main.querySelectorAll('button,input,select,textarea,nav a')).filter(element => {
      const rect = element.getBoundingClientRect();
      return rect.width > 0 && rect.height > 0;
    }).map(element => {
      const target = element instanceof HTMLInputElement && element.type === 'checkbox' ? element.closest('label') ?? element : element;
      const rect = target.getBoundingClientRect();
      return { label: target.textContent?.trim() || element.getAttribute('aria-label') || element.tagName, width: rect.width, height: rect.height };
    })));
    }
    expect(targets.length).toBeGreaterThan(20);
    expect(targets.filter(target => target.width < 44 || target.height < 44)).toEqual([]);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  } finally { await context.close(); }
});

test('touch action fields keep full-size controls across specialized and generic forms', async ({ browser }) => {
  const context = await browser.newContext({ viewport: { width: 360, height: 900 }, isMobile: true, hasTouch: true });
  try {
    const page = await context.newPage();
    await page.goto('http://127.0.0.1:59996/');
    await page.getByRole('button', { name: 'Play / pause', exact: true }).waitFor();
    await page.keyboard.press('q');
    await page.getByRole('button', { name: 'Edit Play / pause', exact: true }).click();
    const dialog = page.getByRole('dialog', { name: 'Edit button', exact: true });
    for (const type of ['write', 'key', 'button', 'debug']) {
  await page.getByRole('tab', { name: 'Action', exact: true }).click();
      await dialog.getByRole('combobox', { name: 'Type', exact: true }).selectOption(type);
      if (type === 'debug') await dialog.getByRole('button', { name: 'Add field', exact: true }).click();
      const small = await dialog.evaluate(root => Array.from(root.querySelectorAll('button,input,select,textarea')).filter(element => {
        const target = element instanceof HTMLInputElement && element.type === 'checkbox' ? element.closest('label') ?? element : element;
        const rect = target.getBoundingClientRect();
        return rect.width > 0 && rect.height > 0 && (rect.width < 44 || rect.height < 44);
      }).map(element => ({ tag: element.tagName, label: element.getAttribute('aria-label') || element.closest('label')?.textContent?.trim(), height: element.getBoundingClientRect().height })));
      expect(small, type).toEqual([]);
    }
  } finally { await context.close(); }
});


test('runtime settings inspect embedded packages and reload without losing config', async ({page,request}) => {
  await page.goto('/');
  await expect(page.getByRole('button', {name:'Play / pause',exact:true})).toBeVisible();
  await page.keyboard.press('Control+,');
  await page.getByRole('tab', { name: 'Runtime and plugins', exact: true }).click();
  const runtime=page.locator('#settings-runtime');
  await expect(runtime.getByText('Runtime status: Ready')).toBeVisible();
  await expect(runtime.getByText(/builtin.obs/)).toBeVisible();
  await runtime.getByRole('button',{name:'Reload plugins',exact:true}).click();
  await expect(runtime.getByText('Runtime status: Ready')).toBeVisible();
  const config=await (await request.get('/api/v2/config')).json();
  expect(config.config.layout.folders[0].label).toBe('Home');
});

test('runtime button state events update presentation without editing config', async ({page,request}) => {
  await page.goto('/');
  const boot=await (await request.get('/api/v2/boot')).json();
  const button=boot.layout.folders[0].buttons.find((b:any)=>b.label==='Play / pause');
  await expect(page.getByRole('button',{name:'Play / pause',exact:true})).toBeVisible();
  // Socket connection is established by the application before event observation.
  await page.waitForTimeout(200);
  const code=`ctx.emit({api_version:2,type:'button.stateChanged',button_id:${JSON.stringify(button.id)},label:'Now playing',active:true});`;
  const result=await (await request.post('/api/v2/commands',{data:{request_id:'dynamic-state',command:{type:'script',language:'javascript',source:{type:'inline',code}}}})).json();
  expect(result.state).toBe('completed');
  await expect(page.getByRole('button',{name:'Now playing',exact:true})).toBeVisible();
  const config=await (await request.get('/api/v2/config')).json();
  expect(config.config.layout.folders[0].buttons.find((b:any)=>b.id===button.id).label).toBe('Play / pause');
});

test('icon registry search and uploaded icon reuse persist through reload', async ({
  page,
  request,
}) => {
  await page.goto('/');
  await expect(page.getByRole('heading', { name: 'Home', exact: true })).toBeVisible();
  await page.keyboard.press('q');
  await page.getByRole('button', { name: 'Edit Play / pause' }).click();
  const dialog = page.getByRole('dialog', { name: 'Edit button' });
  await dialog.getByLabel('Search icons', { exact: true }).fill('camera');
  await dialog.getByRole('button', { name: 'camera', exact: true }).click();
  await expect(dialog.getByLabel('Icon', { exact: true })).toHaveValue('icon:camera');
  await dialog.getByLabel('Upload image', { exact: true }).setInputFiles({
    name: 'custom-icon.svg',
    mimeType: 'image/svg+xml',
    buffer: Buffer.from(
      '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><circle cx="12" cy="12" r="10" fill="red"/></svg>',
    ),
  });
  await expect(dialog.getByLabel('Icon', { exact: true })).toHaveValue(/^asset:/);
  const savedIcon = await dialog.getByLabel('Icon', { exact: true }).inputValue();
  await dialog.getByRole('button', { name: 'Apply to draft' }).click();
  await page.getByRole('button', { name: /^Save( changes)?$/, exact: true }).click();
  await expect(page.getByRole('status').filter({ hasText: /^Saved/ })).toBeVisible();
  await page.reload();
  await expect(page.getByRole('heading', { name: 'Home', exact: true })).toBeVisible();
  await page.keyboard.press('q');
  await page.getByRole('button', { name: 'Edit CPU' }).click();
  await page
    .getByRole('dialog')
    .getByRole('button', { name: `Select uploaded icon ${savedIcon.slice(6)}`, exact: true })
    .click();
  await expect(page.getByRole('dialog').getByLabel('Icon', { exact: true })).toHaveValue(savedIcon);
  await page.getByRole('button', { name: 'Apply to draft' }).click();
  await page.getByRole('button', { name: /^Save( changes)?$/, exact: true }).click();
  await expect(page.getByRole('status').filter({ hasText: /^Saved/ })).toBeVisible();
  const config = await (await request.get('/api/v2/config')).json();
  expect(
    config.config.layout.folders[0].buttons.filter((b: { icon: string }) => b.icon === savedIcon),
  ).toHaveLength(2);
});
