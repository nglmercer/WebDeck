import { stopChild } from '../../../tools/validation/child-process.mjs';
import { test, expect, type Page } from './fixtures';
async function controls(page: Page) {
  await page.locator('.deck-scroll').waitFor();
  await page.keyboard.press('F1');
  await page.getByRole('tab', { name: 'Deck controls', exact: true }).click();
}
async function connect(page: Page) {
  await controls(page);
  await page.getByRole('button', { name: 'Connect Device', exact: true }).click();
}

test('QR invitation connects a separate phone, remembers trust, recovers Wi-Fi and revokes immediately', async ({ page, browser, request }) => {
  await page.goto('/');
  await connect(page);
  await expect(page.getByAltText('QR code to open the deck on your phone').first()).toBeVisible();
  await page.getByText('Connection link', { exact: true }).click();
  const link = page.locator('a[href*="#connect="]');
  const url = (await link.getAttribute('href'))!;
  expect(url).not.toContain('token');
  const phone = await browser.newContext({ viewport: { width: 390, height: 844 } });
  const mobile = await phone.newPage();
  await mobile.goto(url);
  await expect(mobile.getByRole('heading', { name: 'Waiting for the host', exact: true })).toBeVisible();
  expect(await mobile.evaluate(() => location.hash)).toBe('');
  const code = await mobile.locator('.pair-code').textContent();
  const approval = page.locator('.approvals article').filter({ hasText: code!.trim() });
  await expect(approval).toBeVisible({ timeout: 10000 });
  await approval.getByLabel('Expires after').selectOption('2592000');
  await approval.getByRole('button', { name: 'Approve device', exact: true }).click();
  await expect(mobile.getByRole('button', { name: 'Play / pause', exact: true })).toBeVisible();
  await mobile.reload();
  await expect(mobile.getByRole('button', { name: 'Play / pause', exact: true })).toBeVisible();
  const remembered = await phone.storageState();
  await phone.close();
  const returning = await browser.newContext({ storageState: remembered, viewport: { width: 844, height: 390 } });
  const resumed = await returning.newPage();
  await resumed.goto(url.split('#')[0]!);
  await expect(resumed.getByRole('button', { name: 'Play / pause', exact: true })).toBeVisible();
  expect(await resumed.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  await returning.setOffline(true);
  await expect(resumed.getByText('Connection lost. Reconnecting automatically.')).toBeVisible();
  await returning.setOffline(false);
  await expect(resumed.getByText('Connection lost. Reconnecting automatically.')).not.toBeVisible({ timeout: 35000 });
  const token = await resumed.evaluate(() => localStorage.getItem('webdeck.device'));
  const devices = await (await request.get('/api/v2/devices')).json();
  const device = devices.devices.find((d: { name: string }) => d.name === 'My phone');
  expect(device).toBeTruthy();
  await request.delete(`/api/v2/devices/${device.id}`);
  await expect(resumed.getByText('Access expired or was revoked. Request approval on your computer again.')).toBeVisible();
  expect((await returning.request.get(`${url.split('#')[0]}api/v2/boot`, { headers: { Authorization: `Bearer ${token}` } })).status()).toBe(401);
  await returning.close();
});
test('a phone request can be rejected and invitations cannot be replayed', async ({ page, browser, request }) => {
  await page.goto('/');
  await connect(page);
  await page.getByText('Connection link', { exact: true }).click();
  const url = (await page.locator('a[href*="#connect="]').getAttribute('href'))!;
  const phone = await browser.newContext(); const mobile = await phone.newPage();
  await mobile.goto(url);
  const code = await mobile.locator('.pair-code').textContent();
  const approval = page.locator('.approvals article').filter({ hasText: code!.trim() });
  await approval.getByRole('button', { name: 'Reject', exact: true }).click();
  await expect(mobile.getByText('The host rejected this pairing request.')).toBeVisible();
  const invitation = url.split('#connect=')[1]!;
  const replay = await phone.request.post(`${url.split('#')[0]}api/v2/pairing/request`, { data: { name: 'Replay', invitation } });
  expect(replay.status()).toBe(401);
  expect((await request.get('/api/v2/pairing')).ok()).toBe(true);
  await phone.close();
});
test('view-only phone requests permission, host approves, and a new action attempt succeeds', async ({ page, browser, request }) => {
  await page.goto('/'); await connect(page);
  await page.getByText('Connection link', { exact: true }).click();
  const url = (await page.locator('a[href*="#connect="]').getAttribute('href'))!;
  const phone = await browser.newContext(); const mobile = await phone.newPage();
  await mobile.goto(url);
  const code = await mobile.locator('.pair-code').textContent();
  const approval = page.locator('.approvals article').filter({ hasText: code!.trim() });
  await approval.locator('input[type=radio][value=view]').check();
  await approval.getByRole('button', { name: 'Approve device', exact: true }).click();
  await expect(mobile.getByRole('button', { name: 'Play / pause', exact: true })).toBeDisabled();
  await controls(mobile);
  await mobile.getByText('Request Permission', { exact: true }).first().click();
  await mobile.getByRole('switch', { name: 'Audio and media', exact: true }).check();
  await mobile.getByRole('button', { name: 'Request Permission', exact: true }).click();
  const upgrade = page.locator('.approvals article').filter({ hasText: 'This device requests additional access.' });
  await upgrade.getByRole('button', { name: 'Approve device', exact: true }).click();
  await expect(mobile.getByRole('button', { name: 'Play / pause', exact: true })).toBeEnabled();
  await mobile.getByRole('button', { name: 'Close controls', exact: true }).click();
  await mobile.getByRole('button', { name: 'Play / pause', exact: true }).click();
  await expect(mobile.getByRole('status').filter({ hasText: 'Completed' })).toBeVisible();
  const device = (await (await request.get('/api/v2/devices')).json()).devices.at(-1);
  expect(device.capabilities.sort()).toEqual(['audio', 'read']);
  await phone.close();
});

test('trusted phone reconnects after the host process restarts without another approval', async ({ browser, request }) => {
  test.setTimeout(90000);
  const { spawn } = await import('node:child_process');
  const { createServer } = await import('node:net');
  const { mkdtempSync, cpSync, rmSync } = await import('node:fs');
  const { tmpdir } = await import('node:os');
  const path = await import('node:path');
  const reservation = createServer(); await new Promise<void>((resolve) => reservation.listen(0, '127.0.0.1', resolve));
  const port = (reservation.address() as { port: number }).port;
  await new Promise<void>((resolve) => reservation.close(() => resolve()));
  const root = path.resolve('..'), directory = mkdtempSync(path.join(tmpdir(), 'webdeck-reconnect-'));
  cpSync(path.join(root, 'examples/demo-v2/user_uploads'), path.join(directory, 'user_uploads'), { recursive: true });
  const executable = process.env.WEBDECK_ACCEPTANCE_BINARY ?? path.join(root, 'target/debug/webdeck' + (process.platform === 'win32' ? '.exe' : ''));
  let child: ReturnType<typeof spawn> | undefined;
  const start = async () => {
    child = spawn(executable, ['--no-tray', '--port', String(port), '--config-dir', directory], { cwd: root, env: { ...process.env, WEBDECK_FAKE_EFFECTS: '1' }, stdio: 'ignore' });
    await expect.poll(async () => { try { return (await request.get(`http://127.0.0.1:${port}/api/v2/boot`)).status(); } catch { return 0; } }, { timeout: 20000 }).toBe(200);
  };
  const stop = async () => { await stopChild(child); };
  const phone = await browser.newContext();
  try {
    await start();
    const host = `http://127.0.0.1:${port}/api/v2/`;
    expect((await request.post(`${host}connection/start`)).ok()).toBe(true);
    const invitation = await (await request.post(`${host}connection/invitations`)).json();
    const mobile = await phone.newPage(); await mobile.goto(invitation.url);
    await expect(mobile.locator('.pair-code')).toBeVisible();
    const pending = (await (await request.get(`${host}pairing`)).json()).requests[0];
    await request.post(`${host}pairing/${pending.id}/approve`, { data: { name: pending.name, capabilities: ['read', 'audio'], ttl_seconds: 2592000 } });
    await expect(mobile.getByRole('button', { name: 'Play / pause', exact: true })).toBeVisible();
    await stop();
    await expect(mobile.getByText('Connection lost. Reconnecting automatically.')).toBeVisible();
    await start();
    await expect(mobile.getByText('Connection lost. Reconnecting automatically.')).not.toBeVisible({ timeout: 35000 });
    await mobile.reload();
    await expect(mobile.getByRole('button', { name: 'Play / pause', exact: true })).toBeVisible();
    expect((await (await request.get(`${host}pairing`)).json()).requests).toEqual([]);
  } finally { await phone.close(); await stop(); rmSync(directory, { recursive: true, force: true }); }
});

test('two phones pair independently and presence reflects their authenticated sockets', async ({ page, browser, request }) => {
  await page.goto('/'); await connect(page);
  await expect(page.getByAltText('QR code to open the deck on your phone').first()).toBeVisible();
  const invitations = await Promise.all([1, 2].map(async () => (await (await request.post('/api/v2/connection/invitations')).json()).url as string));
  const contexts = await Promise.all(invitations.map(() => browser.newContext()));
  try {
    const mobiles = await Promise.all(contexts.map((context) => context.newPage()));
    await Promise.all(mobiles.map((mobile, index) => mobile.goto(invitations[index]!)));
    for (const mobile of mobiles) {
      const code = await mobile.locator('.pair-code').textContent();
      const approval = page.locator('.approvals article').filter({ hasText: code!.trim() });
      await approval.getByRole('button', { name: 'Approve device', exact: true }).click();
      await expect(mobile.getByRole('button', { name: 'Play / pause', exact: true })).toBeVisible();
    }
    const credentials = await Promise.all(mobiles.map((mobile) => mobile.evaluate(() => sessionStorage.getItem('webdeck.device'))));
    expect(new Set(credentials).size).toBe(2);
    await expect.poll(async () => (await (await request.get('/api/v2/devices/presence')).json()).devices.filter((d: { online: boolean }) => d.online).length).toBe(2);
    await contexts[0]!.close();
    await expect.poll(async () => (await (await request.get('/api/v2/devices/presence')).json()).devices.filter((d: { online: boolean }) => d.online).length).toBe(1);
    await expect(mobiles[1]!.getByRole('button', { name: 'Play / pause', exact: true })).toBeEnabled();
  } finally { await Promise.all(contexts.map((context) => context.close())); }
});

test('a new network origin requires fresh host approval rather than transferring browser credentials', async ({ page, browser, request }) => {
  const { createServer } = await import('node:net');
  await page.goto('/'); await connect(page);
  await page.getByText('Connection link', { exact: true }).click();
  const original = (await page.locator('a[href*="#connect="]').getAttribute('href'))!;
  const phone = await browser.newContext(); const mobile = await phone.newPage();
  const saved = (await (await request.get('/api/v2/network')).json()).settings;
  const reserve = createServer(); await new Promise<void>((resolve) => reserve.listen(0, '127.0.0.1', resolve));
  const port = (reserve.address() as { port: number }).port; await new Promise<void>((resolve) => reserve.close(() => resolve()));
  try {
    await mobile.goto(original);
    let code = await mobile.locator('.pair-code').textContent();
    await page.locator('.approvals article').filter({ hasText: code!.trim() }).getByRole('button', { name: 'Approve device', exact: true }).click();
    await expect(mobile.getByRole('button', { name: 'Play / pause', exact: true })).toBeVisible();
    const oldToken = await mobile.evaluate(() => localStorage.getItem('webdeck.device'));
    expect((await request.put('/api/v2/network', { data: { ...saved, port } })).ok()).toBe(true);
    const invite = await (await request.post('/api/v2/connection/invitations')).json();
    expect(new URL(invite.url).origin).not.toBe(new URL(original).origin);
    await mobile.goto(invite.url);
    await expect(mobile.locator('.pair-code')).toBeVisible();
    expect(await mobile.evaluate(() => localStorage.getItem('webdeck.device'))).toBeNull();
    code = await mobile.locator('.pair-code').textContent();
    await page.locator('.approvals article').filter({ hasText: code!.trim() }).getByRole('button', { name: 'Approve device', exact: true }).click();
    await expect(mobile.getByRole('button', { name: 'Play / pause', exact: true })).toBeVisible();
    expect(await mobile.evaluate(() => localStorage.getItem('webdeck.device'))).not.toBe(oldToken);
  } finally { await phone.close(); await request.put('/api/v2/network', { data: saved }); }
});

test('a phone pairing timeout clears its challenge and offers another request', async ({ page, browser, request }) => {
  await page.goto('/'); await connect(page);
  await page.getByText('Connection link', { exact: true }).click();
  const url = (await page.locator('a[href*="#connect="]').getAttribute('href'))!;
  const phone = await browser.newContext(); const mobile = await phone.newPage();
  try {
    await mobile.clock.install(); await mobile.goto(url);
    await expect(mobile.locator('.pair-code')).toBeVisible();
    await mobile.clock.fastForward(121000);
    await expect(mobile.locator('.pair-code')).toHaveCount(0);
    await expect(mobile.getByRole('button', { name: 'Request access', exact: true })).toBeVisible();
  } finally { await phone.close(); await request.delete(`/api/v2/connection/invitations/${url.split('#connect=')[1]}`); }
});

test('permission rejection and approval work through HTTP polling when realtime is unavailable', async ({ page, browser, request }) => {
  await page.goto('/'); await connect(page);
  const network = await (await request.get('/api/v2/network')).json();
  const grant = await (await request.post('/api/v2/devices', { data: { name: 'HTTP fallback phone', capabilities: ['read'], ttl_seconds: 900 } })).json();
  const phone = await browser.newContext();
  try {
    await phone.route('**/socket.io/**', (route) => route.abort());
    await phone.addInitScript((token) => sessionStorage.setItem('webdeck.device', token), grant.token);
    const mobile = await phone.newPage(); let commands = 0;
    mobile.on('request', (request) => { if (request.method() === 'POST' && request.url().endsWith('/api/v2/commands')) commands++; });
    await mobile.goto(network.url);
    await controls(mobile);
  await mobile.getByText('Request Permission', { exact: true }).first().click();
    await mobile.getByRole('switch', { name: 'Audio and media', exact: true }).check();
    await mobile.getByRole('button', { name: 'Request Permission', exact: true }).click();
    let upgrade = page.locator('.approvals article').filter({ hasText: 'HTTP fallback phone' });
    await upgrade.getByRole('button', { name: 'Reject', exact: true }).click();
    await expect(mobile.getByText('The host rejected this pairing request.')).toBeVisible();
    await expect(mobile.getByRole('button', { name: 'Play / pause', exact: true })).toBeDisabled();
    await mobile.getByRole('button', { name: 'Request Permission', exact: true }).click();
    upgrade = page.locator('.approvals article').filter({ hasText: 'HTTP fallback phone' });
    await upgrade.getByRole('button', { name: 'Approve device', exact: true }).click();
    await expect(mobile.getByRole('button', { name: 'Play / pause', exact: true })).toBeEnabled();
    expect(commands).toBe(0);
    await mobile.getByRole('button', { name: 'Close controls', exact: true }).click();
  await mobile.getByRole('button', { name: 'Play / pause', exact: true }).click();
    await expect(mobile.getByRole('status').filter({ hasText: 'Completed' })).toBeVisible();
    expect(commands).toBe(1);
  } finally { await phone.close(); await request.delete(`/api/v2/devices/${grant.device.id}`); }
});

test('an outstanding QR refreshes automatically when the network address changes', async ({ page, request }) => {
  const { createServer } = await import('node:net');
  await page.goto('/'); await connect(page);
  await page.getByText('Connection link', { exact: true }).click();
  const link = page.locator('a[href*="#connect="]');
  const original = (await link.getAttribute('href'))!;
  const saved = (await (await request.get('/api/v2/network')).json()).settings;
  const reserve = createServer(); await new Promise<void>((resolve) => reserve.listen(0, '127.0.0.1', resolve));
  const port = (reserve.address() as { port: number }).port; await new Promise<void>((resolve) => reserve.close(() => resolve()));
  try {
    expect((await request.put('/api/v2/network', { data: { ...saved, port } })).ok()).toBe(true);
    await expect.poll(() => link.getAttribute('href'), { timeout: 15000 }).not.toBe(original);
    const refreshed = (await link.getAttribute('href'))!;
    expect(new URL(refreshed).port).toBe(String(port));
    expect(refreshed.split('#')[1]).not.toBe(original.split('#')[1]);
    await expect(page.getByAltText('QR code to open the deck on your phone').first()).toBeVisible();
    const old = await (await request.get(`/api/v2/connection/invitations/${original.split('#connect=')[1]}`)).json();
    expect(old.state).toBe('expired');
  } finally { await request.put('/api/v2/network', { data: saved }); }
});
