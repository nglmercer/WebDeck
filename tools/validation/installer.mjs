import assert from 'node:assert/strict';
import { spawn, execFileSync } from 'node:child_process';
import { readFileSync, mkdtempSync, mkdirSync, cpSync, rmSync, existsSync } from 'node:fs';
import path from 'node:path';
import { tmpdir } from 'node:os';
import { createHash } from 'node:crypto';
import { chromium } from '../../frontend/node_modules/@playwright/test/index.mjs';
const artifact = path.resolve(process.argv[2]);
const digest = createHash('sha256').update(readFileSync(artifact)).digest('hex');
assert(readFileSync(`${artifact}.sha256`, 'utf8').startsWith(digest), 'Installer checksum mismatch');
const directory = mkdtempSync(path.join(tmpdir(), 'webdeck-installed-'));
const data = path.join(directory, 'data');
mkdirSync(data);
let mount, install, child, browser, uninstaller;
const run = (command, args) => execFileSync(command, args, { stdio: 'inherit' });
const wait = ms => new Promise(resolve => setTimeout(resolve, ms));
try {
  if (process.platform === 'linux') {
    run('dpkg-deb', ['--extract', artifact, directory]);
    install = path.join(directory, 'opt/webdeck');
    assert(readFileSync(path.join(directory, 'usr/bin/webdeck'), 'utf8').includes('XDG_CONFIG_HOME'));
  } else if (process.platform === 'darwin') {
    mount = path.join(directory, 'volume'); mkdirSync(mount);
    run('hdiutil', ['attach', '-readonly', '-nobrowse', '-mountpoint', mount, artifact]);
    cpSync(path.join(mount, 'WebDeck.app'), path.join(directory, 'WebDeck.app'), { recursive: true });
    run('hdiutil', ['detach', mount]); mount = null;
    const contents = path.join(directory, 'WebDeck.app/Contents');
    run('plutil', ['-lint', path.join(contents, 'Info.plist')]);
    assert(readFileSync(path.join(contents, 'MacOS/launcher'), 'utf8').includes('Library/Application Support/WebDeck'));
    install = path.join(contents, 'Resources/WebDeck');
  } else if (process.platform === 'win32') {
    assert(process.env.CI === 'true', 'Silent installer validation is restricted to disposable CI runners');
    install = path.join(directory, 'application');
    run(artifact, ['/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART', '/SP-', `/DIR=${install}`]);
    uninstaller = path.join(install, 'unins000.exe');
    assert(existsSync(uninstaller), 'Missing Windows uninstaller');
  } else throw Error('Unsupported installer host');
  const binary = path.join(install, process.platform === 'win32' ? 'WebDeck.exe' : 'WebDeck');
  child = spawn(binary, ['--no-tray', '--host', '127.0.0.1', '--port', '59989', '--config-dir', data], { cwd: install, stdio: 'ignore' });
  let spawnError; child.on('error', error => spawnError = error);
  const base = 'http://127.0.0.1:59989';
  let ready = false;
  for (let attempt = 0; attempt < 120; attempt++) {
    if (spawnError) throw spawnError;
    if (child.exitCode !== null) throw Error(`Installed host exited ${child.exitCode}`);
    try { ready = (await fetch(`${base}/api/v2/boot`)).ok; } catch {}
    if (ready) break;
    await wait(100);
  }
  assert(ready, 'Installed host failed readiness');
  browser = await chromium.launch({ headless: true, ...(process.env.WEBDECK_CHROMIUM ? { executablePath: process.env.WEBDECK_CHROMIUM } : existsSync('/usr/bin/chromium') ? { executablePath: '/usr/bin/chromium' } : {}), args: ['--no-sandbox'] });
  const page = await browser.newPage(); const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  await page.goto(base);
  await page.getByRole('button', { name: 'Play / pause', exact: true }).waitFor();
  await page.keyboard.press('q');
  await page.getByRole('button', { name: 'Done', exact: true }).waitFor();
  assert.deepEqual(errors, [], 'Installed UI browser errors');
  assert(existsSync(path.join(data, 'config.json')), 'Missing external user configuration');
  console.log(`Installer validated: ${path.basename(artifact)}; native host and packaged UI; no desktop effects invoked`);
} finally {
  await browser?.close();
  if (child && child.exitCode === null) {
    const exited = new Promise(resolve => child.once('exit', resolve));
    child.kill('SIGINT'); await Promise.race([exited, wait(2000)]);
    if (child.exitCode === null) { child.kill('SIGKILL'); await exited; }
  }
  if (uninstaller && existsSync(uninstaller)) run(uninstaller, ['/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART']);
  if (mount) run('hdiutil', ['detach', mount]);
  rmSync(directory, { recursive: true, force: true });
}
