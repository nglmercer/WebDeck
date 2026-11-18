import { test as base, expect } from '@playwright/test';
import { spawn } from 'node:child_process';
import { cpSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { createServer } from 'node:net';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { stopChild } from '../../../tools/validation/child-process.mjs';
export * from '@playwright/test';

// Every test owns its host, including pairing limits, sessions, uploads and config.
export const test = base.extend({
  baseURL: async ({}, use) => {
    const reservation = createServer();
    await new Promise<void>((resolve) => reservation.listen(0, '127.0.0.1', resolve));
    const port = (reservation.address() as { port: number }).port;
    await new Promise<void>((resolve) => reservation.close(() => resolve()));
    const root = path.resolve('..');
    const directory = mkdtempSync(path.join(tmpdir(), 'webdeck-acceptance-'));
    cpSync(path.join(root, 'examples/demo-v2/user_uploads'), path.join(directory, 'user_uploads'), {
      recursive: true,
    });
    const config = JSON.parse(readFileSync(path.join(root, 'webdeck/config_default.json'), 'utf8'));
    config.layout.folders[0].buttons.push({
      id: 'work-link',
      label: 'Work folder',
      icon: '▦',
      color: '#6654e8',
      action: { type: 'folder', folder_id: 'work' },
      extensions: {},
    });
    config.layout.folders.push({ id: 'work', label: 'Work', buttons: [], extensions: {} });
    writeFileSync(path.join(directory, 'config.json'), JSON.stringify(config));
    const executable =
      process.env.WEBDECK_ACCEPTANCE_BINARY ??
      path.join(root, 'target/debug/webdeck' + (process.platform === 'win32' ? '.exe' : ''));
    // Keep the host's output: when it fails to start, the last lines explain why.
    let hostOutput = '';
    const capture = (chunk: Buffer | string) => {
      hostOutput = (hostOutput + chunk.toString()).slice(-8000);
    };
    const child = spawn(executable, ['--no-tray', '--host', '127.0.0.1', '--port', String(port)], {
      cwd: root,
      env: { ...process.env, WEBDECK_CONFIG_DIR: directory, WEBDECK_FAKE_EFFECTS: '1' },
      stdio: ['ignore', 'pipe', 'pipe'],
    });
    child.stdout?.on('data', capture);
    child.stderr?.on('data', capture);
    let spawnError: Error | undefined;
    child.on('error', (error) => (spawnError = error));
    const url = `http://127.0.0.1:${port}`;
    try {
      await expect
        .poll(
          async () => {
            if (spawnError) throw spawnError;
            if (child.exitCode !== null || child.signalCode !== null)
              throw Error(
                `Acceptance host exited before readiness (code ${child.exitCode ?? child.signalCode})` +
                  (hostOutput.trim() ? `:\n${hostOutput.trim()}` : ''),
              );
            try {
              return (await fetch(`${url}/api/v2/boot`)).ok;
            } catch {
              return false;
            }
          },
          { timeout: 30000 },
        )
        .toBe(true);
      await use(url);
    } finally {
      await stopChild(child);
      rmSync(directory, { recursive: true, force: true });
    }
  },
});
