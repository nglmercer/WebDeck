import assert from 'node:assert/strict';
import test from 'node:test';
import { formatHostDiagnostics } from './acceptance-host.mjs';

function diagnostics(exitCode, signalCode = null, output = '') {
  return formatHostDiagnostics({
    child: { exitCode, signalCode },
    executable: 'C:\\work\\target\\debug\\webdeck.exe',
    args: ['--no-tray', '--host', '127.0.0.1', '--port', '5000'],
    env: { WEBDECK_FAKE_EFFECTS: '1' },
    output,
  });
}

test('startup diagnostics decode the reported Windows exit code', () => {
  const message = diagnostics(3221225785);
  assert.match(message, /^Acceptance host exited before readiness \(code 3221225785\)/);
  assert.match(message, /exitCode: 3221225785/);
  // 3221225785 is 0xC0000139, not 0xC0000409: a DLL entry point was missing.
  assert.match(message, /hexExitCode: 0xC0000139 \(STATUS_ENTRYPOINT_NOT_FOUND \(missing DLL export/);
  assert.match(message, /signalCode: none/);
  assert.match(message, /command: "C:\\\\work\\\\target\\\\debug\\\\webdeck\.exe" "--no-tray"/);
});

test('startup diagnostics keep the fast-fail code distinct', () => {
  assert.match(diagnostics(0xC0000409), /hexExitCode: 0xC0000409 \(STATUS_STACK_BUFFER_OVERRUN/);
});

test('startup diagnostics include signal, captured output and environment', () => {
  const message = diagnostics(null, 'SIGTERM', 'WebDeck: bad config\n');
  assert.match(message, /\(code SIGTERM\)/);
  assert.match(message, /hexExitCode: none/);
  assert.match(message, /WEBDECK_FAKE_EFFECTS=1/);
  assert.match(message, /host output:\nWebDeck: bad config/);
});
