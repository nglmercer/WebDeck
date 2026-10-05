import { chromium, expect } from '../../frontend/node_modules/@playwright/test/index.mjs';
import { mkdtemp, cp, mkdir, rm, rename, writeFile } from 'node:fs/promises';
import { existsSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawn, execFileSync } from 'node:child_process';

const root = fileURLToPath(new URL('../../', import.meta.url));
const output = path.resolve(root, process.env.WEBDECK_DEMO_OUTPUT ?? 'dist/demo-media');
const port = process.env.WEBDECK_DEMO_PORT ?? '59995';
const baseURL = `http://127.0.0.1:${port}`;
const short = process.argv.includes('--short');
if (!process.argv.includes('--skip-build')) {
  execFileSync(process.execPath, ['node_modules/vite/bin/vite.js', 'build'], { cwd: path.join(root, 'frontend'), stdio: 'inherit' });
  execFileSync('cargo', ['build', '--locked', '--bin', 'webdeck'], { cwd: root, stdio: 'inherit' });
}
await mkdir(output, { recursive: true });
const directory = await mkdtemp(path.join(tmpdir(), 'webdeck-recording-'));
await cp(path.join(root, 'examples/demo-v2/config.json'), path.join(directory, 'config.json'));
await cp(path.join(root, 'examples/demo-v2/user_uploads'), path.join(directory, 'user_uploads'), { recursive: true });
const executable = process.env.WEBDECK_ACCEPTANCE_BINARY ?? path.join(process.env.CARGO_TARGET_DIR ? path.resolve(root, process.env.CARGO_TARGET_DIR) : path.join(root, 'target'), 'debug', `webdeck${process.platform === 'win32' ? '.exe' : ''}`);
const child = spawn(executable, ['--no-tray', '--host', '127.0.0.1', '--config-dir', directory, '--port', port], { cwd: root, env: { ...process.env, WEBDECK_FAKE_EFFECTS: '1' }, stdio: ['ignore', 'pipe', 'pipe'] });
let serverLog = '';
child.stdout.on('data', data => serverLog += data);
child.stderr.on('data', data => serverLog += data);
let launchError;
child.on('error', error => launchError = error);
let browser;
const files = [];
const errors = [];
const pause = milliseconds => new Promise(resolve => setTimeout(resolve, milliseconds));
try {
  let ready = false;
  for (let attempt = 0; attempt < 100; attempt++) {
    if (launchError) throw launchError;
    if (child.exitCode !== null) throw Error(`Demo server exited: ${serverLog}`);
    try { ready = (await fetch(`${baseURL}/api/v2/boot`)).ok; } catch { /* Server is still starting. */ }
    if (ready) break;
    await pause(300);
  }
  if (!ready) throw Error(`Demo server did not become ready: ${serverLog}`);
  browser = await chromium.launch({ headless: true, ...(process.env.WEBDECK_CHROMIUM ? { executablePath: process.env.WEBDECK_CHROMIUM } : existsSync('/usr/bin/chromium') ? { executablePath: '/usr/bin/chromium' } : {}), args: ['--no-sandbox'] });
  async function record(name, viewport, mobile, tour) {
    const context = await browser.newContext({ baseURL, viewport, isMobile: mobile, hasTouch: mobile, recordVideo: { dir: output, size: viewport }, reducedMotion: 'reduce' });
    await context.addInitScript(() => localStorage.setItem('webdeck.hint.dismissed', '1'));
    const page = await context.newPage();
    page.on('pageerror', error => errors.push(error.message));
    const video = page.video();
    await context.tracing.start({ screenshots: true, snapshots: true });
    async function shot(label) {
      await page.evaluate(() => document.fonts.ready);
      await expect(page.getByRole('alert')).toHaveCount(0);
      expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
      const filename = `${name}-${label}.png`;
      await page.screenshot({ path: path.join(output, filename), fullPage: true });
      files.push(filename);
      // Deliberate dwell for viewers, rather than a readiness delay.
      await pause(1600);
    }
    try {
      await page.goto('/');
      await expect(page.getByRole('button', { name: 'Folder 1', exact: true })).toBeVisible();
      await tour(page, shot);
      await context.tracing.stop({ path: path.join(output, `${name}-trace.zip`) });
    } finally { await context.close(); }
    const filename = `${name}-demo.webm`;
    await rename(await video.path(), path.join(output, filename));
    files.push(filename, `${name}-trace.zip`);
    try {
      execFileSync('ffmpeg', ['-y', '-i', path.join(output, filename), '-c:v', 'libx264', '-preset', 'fast', '-crf', '20', '-pix_fmt', 'yuv420p', '-movflags', '+faststart', path.join(output, `${name}-demo.mp4`)], { stdio: 'ignore' });
      files.push(`${name}-demo.mp4`);
    } catch { console.log(`MP4 conversion unavailable; Playwright WebM recording is saved: ${filename}`); }
  }
  await record('desktop', { width: 1440, height: 900 }, false, async (page, shot) => {
    await shot('01-home');
    await page.getByRole('button', { name: 'Spotify', exact: true }).click();
    await shot('02-media-folder');
    await page.getByRole('button', { name: 'Back', exact: true }).click();
    await page.keyboard.press('q');
    await shot('03-edit-deck');
    await page.getByRole('button', { name: 'Edit Folder 1', exact: true }).click();
    const dialog = page.getByRole('dialog', { name: 'Edit button', exact: true });
    await expect(dialog).toBeVisible();
    for (const tab of short ? ['Appearance'] : ['Content', 'Appearance', 'Action']) {
      await dialog.getByRole('tab', { name: tab, exact: true }).click();
      await shot(`04-editor-${tab.toLowerCase()}`);
    }
    await dialog.getByRole('button', { name: 'Cancel', exact: true }).click();
    await page.keyboard.press('q');
    await page.keyboard.press('Control+,');
    for (const tab of short ? ['Appearance'] : ['Appearance', 'Integrations', 'Devices', 'Backups', 'Runtime and plugins', 'Connection']) {
      await page.getByRole('tab', { name: tab, exact: true }).click();
      await shot(`05-settings-${tab.toLowerCase().replaceAll(' ', '-')}`);
    }
    await page.getByRole('button', { name: 'Back to deck', exact: true }).click();
    if (!short) { await page.keyboard.press('F1'); await shot('06-shortcuts'); await page.keyboard.press('Escape'); }
    await shot('07-home-final');
  });
  await record('mobile', { width: 390, height: 844 }, true, async (page, shot) => {
    await shot('01-portrait');
    await page.locator('.deck-scroll').evaluate(el => el.scrollTop = el.scrollHeight);
    await shot('01-portrait-last-row');
    await page.locator('.deck-scroll').evaluate(el => el.scrollTop = 0);
    await page.setViewportSize({ width: 844, height: 390 });
    await shot('02-landscape');
    await page.setViewportSize({ width: 390, height: 844 });
    await page.getByRole('button', { name: 'Edit', exact: true }).click();
    await shot('03-edit-deck');
    await page.getByRole('button', { name: 'Edit Folder 1', exact: true }).click();
    await shot('04-button-editor');
    await page.getByRole('button', { name: 'Cancel', exact: true }).click();
    await page.keyboard.press('q');
    await page.getByRole('button', { name: 'Settings', exact: true }).click();
    await shot('05-settings');
  });
  if (errors.length) throw Error(`Browser errors: ${errors.join('; ')}`);
  await writeFile(path.join(output, 'manifest.json'), JSON.stringify({ generated_at: new Date().toISOString(), simulated_desktop_effects: true, configuration: 'examples/demo-v2/config.json', browser: 'Chromium / Playwright', files }, null, 2));
  console.log(`Saved ${files.length} demo artifacts to ${output}`);
} finally {
  await browser?.close();
  if (child.exitCode === null && !launchError) {
    const exited = new Promise(resolve => child.once('exit', resolve));
    child.kill('SIGINT');
    const timer = setTimeout(() => child.kill('SIGKILL'), 5000);
    await exited;
    clearTimeout(timer);
  }
  await rm(directory, { recursive: true, force: true });
}
