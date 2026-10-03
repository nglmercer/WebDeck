import { mkdtempSync, cpSync, readFileSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { spawn } from 'node:child_process';
const root = path.resolve('..');
const directory = mkdtempSync(path.join(tmpdir(), 'webdeck-acceptance-'));
cpSync(path.join(root, 'examples/demo-v2/user_uploads'), path.join(directory, 'user_uploads'), { recursive: true });
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
const child = spawn(executable, ['--no-tray', '--host', '127.0.0.1', '--port', '59996'], {
  cwd: root,
  env: { ...process.env, WEBDECK_CONFIG_DIR: directory, WEBDECK_FAKE_EFFECTS: '1' },
  stdio: 'inherit',
});
let stopping = false;
function stop() {
  if (stopping) return;
  stopping = true;
  child.kill('SIGINT');
  setTimeout(() => child.kill('SIGKILL'), 5000).unref();
}
process.on('SIGTERM', stop);
process.on('SIGINT', stop);
child.on('error', (e) => {
  console.error(e);
  rmSync(directory, { recursive: true, force: true });
  process.exit(1);
});
child.on('exit', (code) => {
  rmSync(directory, { recursive: true, force: true });
  process.exit(stopping ? 0 : (code ?? 1));
});
