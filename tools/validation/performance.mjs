import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdtempSync, readFileSync, writeFileSync, readdirSync, rmSync, mkdirSync, existsSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { createRequire } from 'node:module';
import { gzipSync } from 'node:zlib';

const root = path.resolve(import.meta.dirname, '../..');
const baselineDirectory = process.env.WEBDECK_FRONTEND_BASELINE;
const data = mkdtempSync(path.join(tmpdir(), 'webdeck-performance-'));
const base = 'http://127.0.0.1:59993';
const child = spawn(path.join(root, 'target/debug/webdeck'),
  ['--no-tray', '--host', '127.0.0.1', '--port', '59993'], {
    cwd: baselineDirectory ?? root, env: { ...process.env, WEBDECK_CONFIG_DIR: data, WEBDECK_FAKE_EFFECTS: '1' },
    stdio: 'ignore',
  });
let spawnError;
child.on('error', error => { spawnError = error; });
const exited = new Promise(resolve => child.once('exit', resolve));
const wait = ms => new Promise(resolve => setTimeout(resolve, ms));
const distribution = values => {
  const sorted = [...values].sort((a, b) => a - b);
  return { samples: sorted.length, p50_ms: sorted[Math.floor(sorted.length * .5)],
    p95_ms: sorted[Math.min(sorted.length - 1, Math.floor(sorted.length * .95))], values_ms: values };
};
async function json(endpoint, options) {
  const response = await fetch(base + endpoint, options);
  if (!response.ok) throw new Error(`${endpoint}: ${response.status}`);
  return response.json();
}
let browser;
try {
  const started = performance.now();
  for (;;) {
    if (spawnError) throw spawnError;
    if (child.exitCode !== null) throw new Error('Server exited before readiness');
    try { await json('/api/v2/boot'); break; } catch {}
    if (performance.now() - started > 10000) throw new Error('Server readiness timed out');
    await wait(25);
  }
  const startup = performance.now() - started;
  const form = new FormData();
  form.append('file', new Blob([Buffer.from(
    'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+jK1cAAAAASUVORK5CYII=',
    'base64')], { type: 'image/png' }), 'shared.png');
  const icon = await json('/api/v2/assets', { method: 'POST', body: form });
  const require = createRequire(path.join(root, 'frontend/package.json'));
  browser = await require('@playwright/test').chromium.launch({ headless: true,
    executablePath: process.env.WEBDECK_CHROMIUM ?? (existsSync('/usr/bin/chromium') ? '/usr/bin/chromium' : undefined),
    args: ['--no-sandbox'] });
  const workloads = [];
  for (const count of [48, 1024]) {
    const snapshot = await json('/api/v2/config');
    const config = snapshot.config;
    config.layout.columns = 8;
    config.layout.rows = 6;
    config.layout.folders = [{ id: 'home', label: 'Home', extensions: {}, buttons:
      Array.from({ length: count }, (_, index) => ({
        id: `tile-${index}`, label: `Tile ${index}`, icon: `asset:${icon.id}`, color: '', extensions: {},
        action: index === 0 ? { type: 'usage' } : { type: 'none' },
      })) }];
    await json('/api/v2/config', { method: 'PUT', headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ revision: snapshot.revision, config }) });
    const boot = [];
    for (let sample = 0; sample < 20; sample++) {
      const start = performance.now();
      await json('/api/v2/boot');
      boot.push(performance.now() - start);
    }
    const loads = [], assetRequests = [], pageErrors = [], enterEditing = [], leaveEditing = [], edits = [], reverts = [];
    let polling;
    for (let sample = 0; sample < 5; sample++) {
      const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
      try {
        const page = await context.newPage();
        let assets = 0, usage = 0;
        page.on('request', request => {
          if (new URL(request.url()).pathname.startsWith('/api/v2/assets/')) assets++;
          if (new URL(request.url()).pathname === '/api/v2/usage') usage++;
        });
        page.on('pageerror', error => pageErrors.push(error.message));
        const start = performance.now();
        await page.goto(base);
        await page.getByRole('heading', { name: 'Home', exact: true }).waitFor();
        await page.locator('.deck-button img').nth(count - 1).waitFor();
        assert.equal(await page.locator('.deck-button img').count(), count);
        await page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
        loads.push(performance.now() - start);
        assetRequests.push(assets);
        assert.equal(assets, baselineDirectory ? count : 1, 'Asset request count must match the measured frontend algorithm');
        if (baselineDirectory) continue;
        const editStarted = performance.now();
        await page.keyboard.press('q');
        await page.getByRole('region', { name: 'Deck editor' }).waitFor();
        await page.getByRole('button', { name: 'Edit Tile 0', exact: true }).waitFor();
        enterEditing.push(performance.now() - editStarted);
        const field = page.getByLabel('Folder name', { exact: true });
        const editChanged = performance.now();
        await field.fill('Temporary benchmark edit');
        await page.waitForFunction(() => document.querySelector('.save-status')?.textContent === 'Unsaved changes');
        edits.push(performance.now() - editChanged);
        const editReverted = performance.now();
        await field.fill('Home');
        await page.waitForFunction(() => document.querySelector('.save-status')?.textContent === 'All changes saved');
        assert.equal(await page.getByRole('button', { name: 'Save changes', exact: true }).isDisabled(), true);
        reverts.push(performance.now() - editReverted);
        await page.getByRole('region', { name: 'Control deck', exact: true }).focus();
        const leaveStarted = performance.now();
        await page.keyboard.press('q');
        await page.getByRole('region', { name: 'Deck editor' }).waitFor({ state: 'hidden' });
        leaveEditing.push(performance.now() - leaveStarted);
        if (sample === 0) {
          // Observe a real polling response, then measure a bounded absence window in settings.
          await page.waitForResponse(response => new URL(response.url()).pathname === '/api/v2/usage' && usage >= 2);
          const deckRequests = usage;
          await page.keyboard.press('Control+,');
          await page.getByRole('heading', { name: 'Settings', exact: true }).waitFor();
          const settingsRequests = usage;
          const nextPoll = await page.waitForRequest(request => new URL(request.url()).pathname === '/api/v2/usage',
            { timeout: 1500 }).catch(error => {
              if (error.name === 'TimeoutError') return null;
              throw error;
            });
          assert.equal(nextPoll, null, 'Settings must stop usage polling');
          polling = { observed_deck_requests: deckRequests, settings_observation_ms: 1500,
            transition_requests: settingsRequests - deckRequests,
            additional_settings_requests: usage - settingsRequests };
        }
      } finally { await context.close(); }
    }
    assert.deepEqual(pageErrors, []);
    workloads.push({ buttons: count, boot: distribution(boot), usable_ui: distribution(loads),
      keyboard_enter_editing: distribution(enterEditing), keyboard_leave_editing: distribution(leaveEditing),
      folder_edit: distribution(edits), folder_revert: distribution(reverts),
      unique_icons: 1, icon_references: count, asset_requests: assetRequests, polling, page_errors: pageErrors });
  }
  const commands = [];
  for (let sample = 0; sample < 50; sample++) {
    const start = performance.now();
    await json('/api/v2/commands', { method: 'POST', headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ request_id: `benchmark-${sample}`, command: { type: 'debug', data: {} } }) });
    commands.push(performance.now() - start);
  }
  const assets = path.join(baselineDirectory ?? root, 'frontend/dist/assets');
  const javascript = readdirSync(assets).filter(name => name.endsWith('.js')).map(name => readFileSync(path.join(assets, name)));
  const report = {
    frontend_source: baselineDirectory ? readFileSync(path.join(baselineDirectory, 'comparison-origin.txt'), 'utf8').trim() : 'current worktree',
    profile: 'development server; production UI; fake effects; isolated config; fresh browser contexts; 1440x900 viewport; shared uploaded PNG',
    node: process.version, chromium: browser.version(), startup_to_boot_ms: startup,
    workloads, command: distribution(commands),
    bundle_js_bytes: javascript.reduce((sum, bytes) => sum + bytes.length, 0),
    bundle_gzip_bytes: javascript.reduce((sum, bytes) => sum + gzipSync(bytes).length, 0),
    comparison: 'Workloads are comparable within this run. Keyboard timings include Playwright input and readiness observation overhead. Historical default-deck reports differ; no historical UI speed claim.',
  };
  const output = process.env.WEBDECK_PERFORMANCE_OUTPUT;
  if (output) { mkdirSync(path.dirname(output), { recursive: true }); writeFileSync(output, JSON.stringify(report, null, 2) + '\n'); }
  console.log(JSON.stringify(report, null, 2));
} finally {
  await browser?.close();
  child.kill('SIGINT');
  await Promise.race([exited, wait(2000)]);
  if (child.exitCode === null && !spawnError) { child.kill('SIGKILL'); await exited; }
  rmSync(data, { recursive: true, force: true });
}
