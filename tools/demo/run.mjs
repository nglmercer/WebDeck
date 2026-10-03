import { mkdtemp, cp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawn, execFileSync } from 'node:child_process';
const root = fileURLToPath(new URL('../../', import.meta.url));
execFileSync(process.execPath, ['node_modules/vite/bin/vite.js', 'build'], { cwd: path.join(root, 'frontend'), stdio: 'inherit' });
execFileSync('cargo', ['build', '--locked', '--bin', 'webdeck'], { cwd: root, stdio: 'inherit' });
const directory = await mkdtemp(path.join(tmpdir(), 'webdeck-v2-demo-'));
await cp(path.join(root, 'examples/demo-v2/config.json'), path.join(directory, 'config.json'));
await cp(path.join(root, 'examples/demo-v2/user_uploads'), path.join(directory, 'user_uploads'), { recursive: true });
const child = spawn(path.join(process.env.CARGO_TARGET_DIR ? path.resolve(root, process.env.CARGO_TARGET_DIR) : path.join(root, 'target'), 'debug', 'webdeck' + (process.platform === 'win32' ? '.exe' : '')), [ '--no-tray', '--config-dir', directory, '--port', process.env.WEBDECK_DEMO_PORT ?? '5000'], {
  cwd: root,
  env: { ...process.env, WEBDECK_FAKE_EFFECTS: '1' },
  stdio: 'inherit',
});
console.log('V2 demo: isolated temporary configuration; desktop effects are simulated.');
for (const signal of ['SIGINT', 'SIGTERM']) process.on(signal, () => child.kill(signal));
child.once('error', async error => { console.error(error); await rm(directory, { recursive: true, force: true }); process.exitCode = 1; });
child.once('close', async code => { await rm(directory, { recursive: true, force: true }); process.exitCode = code ?? 0; });
