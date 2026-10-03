import assert from 'node:assert/strict';
import { readFileSync, readdirSync } from 'node:fs';
import path from 'node:path';
import { gzipSync } from 'node:zlib';
const root = path.resolve(import.meta.dirname, '../..');
const budgets = JSON.parse(readFileSync(path.join(root, 'tools/validation/performance-budgets.json')));
const directory = path.join(root, 'frontend/dist/assets');
const files = readdirSync(directory).filter(name => name.endsWith('.js'));
assert(files.length > 0, 'Build the production frontend before checking its budget');
const bytes = files.map(name => readFileSync(path.join(directory, name)));
const raw = bytes.reduce((sum, file) => sum + file.length, 0);
const gzip = bytes.reduce((sum, file) => sum + gzipSync(file).length, 0);
assert(raw <= budgets.bundle_js_bytes, `JavaScript budget exceeded: ${raw} > ${budgets.bundle_js_bytes}`);
assert(gzip <= budgets.bundle_gzip_bytes, `Gzip budget exceeded: ${gzip} > ${budgets.bundle_gzip_bytes}`);
if (process.argv[2]) {
  const report = JSON.parse(readFileSync(process.argv[2]));
  assert.equal(report.bundle_js_bytes, raw, 'Performance report does not match the current JavaScript build');
  assert.equal(report.bundle_gzip_bytes, gzip, 'Performance report does not match the current compressed build');
  for (const [count, limit] of Object.entries(budgets.usable_ui_p95_ms)) {
    const workload = report.workloads.find(workload => workload.buttons === Number(count));
    assert(workload, `Missing ${count}-button workload`);
    assert(workload.usable_ui.samples >= 5, 'At least five UI samples are required');
    assert(workload.usable_ui.p95_ms <= limit, `${count}-button UI exceeds local budget`);
    assert(workload.asset_requests.length >= 5, 'At least five asset observations are required');
    assert(workload.asset_requests.every(count => count === 1), 'Shared icon request regression');
    assert(workload.polling.observed_deck_requests >= 2, 'Periodic polling was not observed');
    assert.equal(workload.polling.additional_settings_requests, 0, 'Settings polling regression');
  }
}
console.log(`Performance budget passed: ${raw} bytes JavaScript / ${gzip} bytes gzip${process.argv[2] ? '; local UI/asset/polling report checked' : ''}.`);
