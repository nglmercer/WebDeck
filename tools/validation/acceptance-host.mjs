// Shared startup path for the Playwright acceptance fixture and the CI
// preflight. Keeping one implementation means a host that passes the preflight
// is the same host (binary, arguments, environment and readiness URL) the tests
// exercise, and a startup failure produces the same diagnostics everywhere.
import { spawn } from 'node:child_process';
import { cpSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { createServer } from 'node:net';
import { release, tmpdir } from 'node:os';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { stopChild } from './child-process.mjs';

const root = path.resolve(import.meta.dirname, '../..');

const WINDOWS_STATUS_HINTS = {
  '0xC0000005': 'STATUS_ACCESS_VIOLATION',
  '0xC00000FD': 'STATUS_STACK_OVERFLOW',
  // 3221225785 is this code, not 0xC0000409: a DLL export could not be
  // resolved. A common Rust cause is rfd/muda importing ComCtl32 v6's
  // TaskDialogIndirect without embedding the v6 application manifest.
  '0xC0000139':
    'STATUS_ENTRYPOINT_NOT_FOUND (missing DLL export; often ComCtl32 v6 TaskDialogIndirect without a manifest)',
  '0xC0000374': 'STATUS_HEAP_CORRUPTION',
  '0xC0000409': 'STATUS_STACK_BUFFER_OVERRUN / __fastfail',
};

/** Pick a free loopback port, then release it immediately for the host. */
export async function reservePort() {
  const reservation = createServer();
  await new Promise((resolve) => reservation.listen(0, '127.0.0.1', resolve));
  const port = reservation.address().port;
  await new Promise((resolve) => reservation.close(resolve));
  return port;
}

/** Describe why a host never became ready, including the platform crash code. */
export function formatHostDiagnostics({
  child,
  executable,
  args,
  env,
  output,
  spawnError,
  reason = 'Acceptance host exited before readiness',
}) {
  const code = spawnError ? null : child.exitCode;
  const hex =
    code === null || code === undefined
      ? 'none'
      : `0x${(code >>> 0).toString(16).toUpperCase().padStart(8, '0')}`;
  // Keep the original prefix intact so existing failure reports still match.
  const headline = spawnError
    ? `${reason}: ${spawnError.message}`
    : `${reason} (code ${code ?? child.signalCode ?? 'unknown'})`;
  // Windows crash codes are unambiguous even when read from another platform's
  // CI report, so annotate them regardless of where the diagnostics are viewed.
  const hint = WINDOWS_STATUS_HINTS[hex];
  const command = [executable, ...args].map((part) => JSON.stringify(part)).join(' ');
  const envKeys = [
    'WEBDECK_ACCEPTANCE_BINARY',
    'WEBDECK_CONFIG_DIR',
    'WEBDECK_FAKE_EFFECTS',
    'RUST_BACKTRACE',
    'CARGO_TARGET_DIR',
    'PATH',
  ];
  return [
    headline,
    `exitCode: ${code ?? 'none'}`,
    `hexExitCode: ${hex}${hint ? ` (${hint})` : ''}`,
    `signalCode: ${child.signalCode ?? 'none'}`,
    `command: ${command}`,
    `cwd: ${process.cwd()}`,
    `executable: ${executable}`,
    `node: ${process.version} (${process.platform} ${process.arch} ${release()})`,
    'environment:',
    ...envKeys.filter((key) => env[key] !== undefined).map((key) => `${key}=${env[key]}`),
    'host output:',
    output.trim() || '(none)',
  ].join('\n');
}

/** Prepare an isolated config directory and spawn one acceptance host. */
export function createAcceptanceHost(port, envOverrides = {}) {
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
  const args = ['--no-tray', '--host', '127.0.0.1', '--port', String(port)];
  const env = {
    ...process.env,
    ...envOverrides,
    WEBDECK_CONFIG_DIR: directory,
    WEBDECK_FAKE_EFFECTS: '1',
  };

  // Keep the host's output: when it fails to start, the last lines explain why.
  let output = '';
  const capture = (chunk) => {
    output = (output + chunk.toString()).slice(-8000);
  };
  const child = spawn(executable, args, { cwd: root, env, stdio: ['ignore', 'pipe', 'pipe'] });
  child.stdout?.on('data', capture);
  child.stderr?.on('data', capture);
  let spawnError;
  child.on('error', (error) => (spawnError = error));

  return {
    child,
    directory,
    executable,
    args,
    env,
    url: `http://127.0.0.1:${port}`,
    get output() {
      return output;
    },
    get spawnError() {
      return spawnError;
    },
    diagnostics: (reason = 'Acceptance host exited before readiness') =>
      formatHostDiagnostics({ child, executable, args, env, output, spawnError, reason }),
  };
}

/** Poll the readiness URL, failing immediately if the host exits. */
export async function waitForBoot(host, timeoutMs = 30000) {
  const deadline = Date.now() + timeoutMs;
  for (;;) {
    if (host.spawnError) throw Error(host.diagnostics('Acceptance host failed to spawn'));
    if (host.child.exitCode !== null || host.child.signalCode !== null)
      throw Error(host.diagnostics());
    try {
      const response = await fetch(`${host.url}/api/v2/boot`, {
        signal: AbortSignal.timeout(2000),
      });
      if (response.ok) return host.url;
    } catch {
      // Host not listening yet; retry until the deadline.
    }
    if (Date.now() >= deadline) throw Error(host.diagnostics('Acceptance host readiness timed out'));
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
}

async function stopHost(host) {
  await stopChild(host.child);
  rmSync(host.directory, { recursive: true, force: true });
}

/** Standalone CI preflight: prove the host boots before Playwright runs. */
async function preflight() {
  const host = createAcceptanceHost(await reservePort());
  try {
    await waitForBoot(host);
    console.log(`Acceptance host started at ${host.url}`);
    return;
  } catch (error) {
    // STATUS_ENTRYPOINT_NOT_FOUND means a DLL import could not be resolved.
    // Retrying with an empty PATH tells us whether the runner's PATH supplied
    // an incompatible DLL. The original failure is still what gets reported.
    if (process.platform === 'win32') {
      let cleanHost;
      try {
        cleanHost = createAcceptanceHost(await reservePort(), { PATH: '' });
        await waitForBoot(cleanHost, 10000);
        console.error('DIAGNOSIS: host boots with an empty PATH; check DLLs picked up from PATH');
      } catch {
        console.error('DIAGNOSIS: host also fails with an empty PATH');
      } finally {
        if (cleanHost) await stopHost(cleanHost);
      }
    }
    throw error;
  } finally {
    await stopHost(host);
  }
}

if (process.argv[1] && import.meta.url === pathToFileURL(path.resolve(process.argv[1])).href) {
  preflight().catch((error) => {
    console.error(error instanceof Error ? (error.stack ?? error.message) : String(error));
    process.exitCode = 1;
  });
}
