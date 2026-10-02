import { test, expect } from '@playwright/test';
import { io } from 'socket.io-client';
import { readFileSync } from 'node:fs';
import type { CommandEvent } from '../../src/contracts';
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
test.beforeEach(async ({ request }) => {
  const s = await (await request.get('/api/v2/config')).json();
  expect(
    (await request.put('/api/v2/config', { data: { revision: s.revision, config: initial } })).ok(),
  ).toBeTruthy();
});
test('new deck edits typed actions, folders, themes and keeps conflict drafts', async ({
  page,
  request,
}) => {
  const errors: string[] = [];
  page.on('pageerror', (e) => errors.push(e.message));
  await page.goto('/');
  await expect(page.getByRole('heading', { name: 'Home', exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Work folder', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Work', exact: true })).toBeVisible();
  await page.locator('aside').getByRole('button', { name: /Home/ }).click();
  await page.getByRole('button', { name: 'Edit deck' }).click();
  await page.getByRole('button', { name: 'Edit Play / pause' }).click();
  await page.getByLabel('Label', { exact: true }).fill('My music');
  await page.getByRole('button', { name: 'Apply to draft' }).click();
  await page.getByRole('button', { name: /^Save( changes)?$/, exact: true }).click();
  await expect(page.getByRole('status')).toContainText('Saved');
  await page.reload();
  await expect(page.getByRole('button', { name: 'My music', exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Edit deck' }).click();
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
  expect(errors).toEqual([]);
});
test('theme and image uploads, folder creation and settings round trip', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Settings', exact: true }).first().click();
  await page.getByLabel('Upload a theme').setInputFiles({
    name: 'theme.css',
    mimeType: 'text/css',
    buffer: Buffer.from('.brand{--test-theme:loaded}'),
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
  await page.getByLabel('Columns', { exact: true }).fill('3');
  await page.getByRole('button', { name: /^Save( changes)?$/, exact: true }).click();
  await expect(page.getByRole('status')).toContainText('Saved');
  await page.reload();
  await expect(page.locator('.brand')).toHaveCSS('--test-theme', 'loaded');
  await page.getByRole('button', { name: 'Edit deck' }).click();
  await page.getByRole('button', { name: /Add folder/ }).click();
  await page.getByLabel('Folder name').fill('Studio');
  await page.getByRole('button', { name: /^Save( changes)?$/, exact: true }).click();
  await page.reload();
  await expect(page.locator('aside').getByRole('button', { name: /Studio/ })).toBeVisible();
  await page
    .locator('aside')
    .getByRole('button', { name: /Studio/ })
    .click();
  await page.getByRole('button', { name: 'Edit deck' }).click();
  await page.getByRole('button', { name: /^\+ Add button$/ }).click();
  await page.getByLabel('Label', { exact: true }).fill('Image button');
  await page.getByLabel('Upload image').setInputFiles(image);
  await expect(page.getByLabel('Icon', { exact: true })).toHaveValue(/^asset:/);
  await page.getByRole('button', { name: 'Apply to draft' }).click();
  await page.getByRole('button', { name: /^Save( changes)?$/ }).click();
  await page.reload();
  await page
    .locator('aside')
    .getByRole('button', { name: /Studio/ })
    .click();
  await expect(
    page.getByRole('button', { name: 'Image button', exact: true }).locator('img'),
  ).toBeVisible();
});
test('typed realtime correlation, retired routes, revoked sessions and offline commands', async ({
  page,
  request,
}) => {
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
  await page.getByLabel('Connection').selectOption('socket');
  await page.waitForTimeout(300);
  await page.getByRole('button', { name: 'Play / pause', exact: true }).click();
  await expect(page.getByRole('status')).toContainText('Completed');
  await page.context().setOffline(true);
  await page.getByRole('button', { name: 'Play / pause', exact: true }).click();
  await expect(page.getByRole('alert')).toBeVisible();
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
  await expect(page.getByRole('button', { name: 'Edit deck' })).toHaveCount(0);
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
  await page.getByRole('button', { name: 'Usage', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'System usage' })).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Memory', exact: true })).toBeVisible();
  await expect(page.getByRole('progressbar')).toHaveCount(
    2 + v.usage.disks.length + v.usage.gpus.length,
  );
  await page.getByRole('button', { name: 'Deck', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Home', exact: true })).toBeVisible();
});
