import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { mkdtempSync, mkdirSync, readFileSync, writeFileSync, rmSync, existsSync } from 'node:fs';
import { createServer } from 'node:net';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { createRequire } from 'node:module';

const root = path.resolve(import.meta.dirname, '../..');
const binary = path.resolve(process.argv[2] ?? path.join(root, 'target/release', process.platform === 'win32' ? 'webdeck.exe' : 'webdeck'));
const directory = mkdtempSync(path.join(tmpdir(), 'webdeck-embedded-ui-'));
const listener = createServer();
listener.listen(0, '127.0.0.1');
await once(listener, 'listening');
const port = listener.address().port;
await new Promise(resolve => listener.close(resolve));
const child = spawn(binary, ['--no-tray', '--host', '127.0.0.1', '--port', String(port)], {
  cwd: directory,
  env: { ...process.env, WEBDECK_CONFIG_DIR: path.join(directory, 'config') },
  stdio: 'ignore',
});
const exited = once(child, 'exit');
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
  assert(ready, 'Release executable failed to start');
  const translations = await fetch(base + '/api/v2/translations');
  assert.equal(translations.status, 200, 'Embedded translation fallback');
  const dictionary = await translations.json();
  assert(dictionary.languages.includes('en_US'));
  assert(Object.keys(dictionary.translations).length > 0);
  const currentHtml = readFileSync(path.join(root, 'frontend/dist/index.html'));
  const assets = [...currentHtml.toString().matchAll(/(?:src|href)="(\/assets\/[^" ]+)"/g)].map(match => match[1]);
  assert(assets.length > 0);
  for (const mode of ['missing disk files', 'stale disk files']) {
    if (mode === 'stale disk files') {
      for (const url of ['/', ...assets, '/static/icons/icon.ico']) {
        const file = path.join(directory, url.startsWith('/static/') ? url.slice(1) : 'frontend/dist/' + (url === '/' ? 'index.html' : url.slice(1)));
        mkdirSync(path.dirname(file), { recursive: true });
        writeFileSync(file, 'stale UI must not be served');
      }
    }
    for (const url of ['/', ...assets, '/static/icons/icon.ico']) {
      const response = await fetch(base + url);
      assert.equal(response.status, 200, `${mode}: ${url}`);
      assert.equal(response.headers.get('cache-control'), 'no-store');
      const expected = url === '/' ? currentHtml : readFileSync(path.join(root, url.startsWith('/static/') ? url.slice(1) : 'frontend/dist/' + url.slice(1)));
      assert.deepEqual(Buffer.from(await response.arrayBuffer()), expected, `${mode}: ${url}`);
    }
  }
  assert.equal((await fetch(base + '/assets/missing.js')).status, 404);
  const require = createRequire(path.join(root, 'frontend/package.json'));
  browser = await require('@playwright/test').chromium.launch({
    headless: true,
    executablePath: process.env.WEBDECK_CHROMIUM ?? (existsSync('/usr/bin/chromium') ? '/usr/bin/chromium' : undefined),
    args: ['--no-sandbox'],
  });
  const page = await browser.newPage();
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  await page.goto(base);
  await page.getByRole('heading', { name: 'Home', exact: true }).waitFor();
  await page.keyboard.press('q');
  await page.getByRole('button', { name: 'Edit Play / pause', exact: true }).click();
  await page.getByRole('tab', { name: 'Action', exact: true }).click();
  await page.getByRole('button', { name: 'Change action', exact: true }).click();
  const picker = page.getByRole('dialog', { name: 'Select action', exact: true });
  await picker.waitFor();
  assert.equal(await picker.locator('.option svg').count(), await picker.locator('.option').count());
  assert.deepEqual(errors, []);
  console.log('Embedded frontend passed: current HTML, JS, CSS and favicon; missing/stale disk files ignored; Chromium UI loads.');
} finally {
  await browser?.close();
  if (child.exitCode === null) {
    child.kill('SIGINT');
    await Promise.race([exited, wait(2000)]);
    if (child.exitCode === null) { child.kill('SIGKILL'); await exited; }
  }
  rmSync(directory, { recursive: true, force: true });
}
