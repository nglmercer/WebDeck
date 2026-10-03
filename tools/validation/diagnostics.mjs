import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdtempSync, rmSync, mkdirSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';

const root = path.resolve(import.meta.dirname, '../..');
const directory = mkdtempSync(path.join(tmpdir(), 'webdeck-diagnostics-'));
const base = 'http://127.0.0.1:59992';
const child = spawn(path.join(root, 'target/debug/webdeck' + (process.platform === 'win32' ? '.exe' : '')),
  ['--no-tray', '--host', '127.0.0.1', '--port', '59992'], {
    cwd: root, env: { ...process.env, WEBDECK_CONFIG_DIR: directory,
      WEBDECK_FAKE_EFFECTS: '1', WEBDECK_DIAGNOSTICS: '1' }, stdio: ['ignore', 'ignore', 'pipe'],
  });
let stderr = '';
let spawnError;
child.on('error', error => { spawnError = error; });
child.stderr.on('data', bytes => {
  stderr += bytes.toString();
  if (stderr.length > 65536) { spawnError = new Error('Unexpected diagnostic volume'); child.kill(); }
});
const exited = new Promise(resolve => child.once('exit', resolve));
const wait = ms => new Promise(resolve => setTimeout(resolve, ms));
try {
  const deadline = Date.now() + 10000;
  for (;;) {
    if (spawnError) throw spawnError;
    if (child.exitCode !== null) throw new Error('Server exited before readiness');
    try { if ((await fetch(base + '/api/v2/boot')).ok) break; } catch {}
    if (Date.now() >= deadline) throw new Error('Server readiness timed out');
    await wait(25);
  }
  const ids = ['sensitive-request-id', 'secret-token\nscript-source'];
  for (const [index, request_id] of ids.entries()) {
    const response = await fetch(base + '/api/v2/commands', {
      method: 'POST', headers: { 'Content-Type': 'application/json', 'X-Test-Secret': 'private-header' },
      body: JSON.stringify({ request_id, command: { type: 'debug', data: { token: 'private-payload' } } }),
    });
    assert.equal(response.status, index === 0 ? 200 : 400);
    await response.json();
  }
  const terminalEvents = () => stderr.split('\n').flatMap(line => {
    try { const event = JSON.parse(line); return event.event === 'command_terminal' ? [event] : []; }
    catch { return []; }
  });
  const logDeadline = Date.now() + 3000;
  while (terminalEvents().length < 2) {
    if (spawnError) throw spawnError;
    if (Date.now() >= logDeadline) throw new Error('Terminal diagnostics were not observed');
    await wait(10);
  }
  const events = terminalEvents();
  assert.equal(events.length, 2);
  for (const [index, event] of events.entries()) {
    assert.equal(event.request_id_sha256, createHash('sha256').update(ids[index]).digest('hex'));
    assert.equal(event.outcome, index === 0 ? 'completed' : 'rejected');
    assert.equal(event.category, 'read');
    assert(Number.isFinite(event.duration_ms) && event.duration_ms >= 0);
    assert.deepEqual(Object.keys(event).sort(), [
      'event', 'request_id_sha256', 'category', 'duration_ms', 'outcome',
      ...(index === 1 ? ['error_code'] : []),
    ].sort());
  }
  for (const secret of [...ids, 'private-header', 'private-payload']) assert(!stderr.includes(secret));
  const report = { verified_at: new Date().toISOString(), profile: 'isolated development server; fake effects', result: 'passed', terminal_events: events.length, checks: ['completed and rejected outcomes', 'request ID SHA-256 correlation', 'read capability category', 'finite nonnegative duration', 'exact allowlisted event fields', 'raw IDs, payload secret and private header absent'], scope: 'HTTP executor diagnostics; no native effects or durable-log delivery claim' };
  const output = process.env.WEBDECK_DIAGNOSTICS_OUTPUT;
  if (output) { mkdirSync(path.dirname(output), { recursive: true }); writeFileSync(output, JSON.stringify(report, null, 2) + '\n'); }
  console.log('Diagnostic HTTP smoke passed: completed/rejected events and metadata redaction.');
} finally {
  child.kill('SIGINT');
  await Promise.race([exited, wait(2000)]);
  if (child.exitCode === null) { child.kill('SIGKILL'); await exited; }
  rmSync(directory, { recursive: true, force: true });
}
