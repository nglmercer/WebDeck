import { stopChild } from './child-process.mjs';
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { mkdtempSync, rmSync, existsSync } from 'node:fs';
import { createServer } from 'node:net';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { createRequire } from 'node:module';
const root = path.resolve(import.meta.dirname, '../..');
const listener = createServer();
listener.listen(0, '127.0.0.1');
await once(listener, 'listening');
const port = listener.address().port;
await new Promise(resolve => listener.close(resolve));
const directory = mkdtempSync(path.join(tmpdir(), 'webdeck-debug-cwd-'));
const child = spawn(path.join(root, 'target/debug', process.platform === 'win32' ? 'webdeck.exe' : 'webdeck'), ['--no-tray', '--port', String(port)], {
  cwd: path.join(root, 'target/release'), env: { ...process.env, WEBDECK_CONFIG_DIR: directory }, stdio: 'ignore',
});
const wait = ms => new Promise(resolve => setTimeout(resolve, ms));
let browser;
try {
  const base = `http://127.0.0.1:${port}`;
  let ready = false;
  for (let attempt = 0; attempt < 200; attempt++) {
    try { if ((await fetch(base + '/api/v2/boot')).ok) { ready = true; break; } } catch {}
    if (child.exitCode !== null) break;
    await wait(25);
  }
  assert(ready, 'Debug server did not start');
  const response = await fetch(base + '/api/v2/translations');
  assert.equal(response.status, 200, 'Translations must load when started from target/release');
  const dictionary = await response.json();
  assert(dictionary.languages.includes('en_US'));
  assert.equal(dictionary.translations['connections.phoneAccess'], 'Phone access');
  const require = createRequire(path.join(root, 'frontend/package.json'));
  browser = await require('@playwright/test').chromium.launch({ headless: true, executablePath: process.env.WEBDECK_CHROMIUM ?? (existsSync('/usr/bin/chromium') ? '/usr/bin/chromium' : undefined), args: ['--no-sandbox'] });
  const page = await browser.newPage();
  await page.goto(base);
  await page.getByRole('heading', { name: 'Home', exact: true }).waitFor();
  await page.getByRole('button', { name: 'Settings', exact: true }).click();
  await page.getByRole('tab', { name: 'Devices & Connections', exact: true }).click();
  await page.getByRole('heading', { name: 'Devices & Connections', exact: true }).waitFor();
  console.log('Debug working-directory regression passed: translations, deck and device settings load from target/release.');
} finally {
  await browser?.close();
  await stopChild(child);
  rmSync(directory, { recursive: true, force: true });
}
