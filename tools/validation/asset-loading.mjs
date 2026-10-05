import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { writeFileSync, readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
const require = createRequire(new URL('../../frontend/package.json', import.meta.url));
const { transformWithOxc } = await import(require.resolve('vite'));
const source = readFileSync(new URL('../../frontend/src/lib/assets.ts', import.meta.url), 'utf8');
const { code: outputText } = await transformWithOxc(source, 'assets.ts');
const { AssetCache } = await import('data:text/javascript;base64,' + Buffer.from(outputText).toString('base64'));

// Reproduce baseline App.svelte assets() traversal, using the same controlled
// asynchronous asset provider for both algorithms. This measures algorithms,
// not HTTP throughput or historical page-load latency.
const baselineCommit = execFileSync('git', ['rev-parse', 'HEAD'], { encoding: 'utf8' }).trim();
const baselineSource = execFileSync('git', ['show', 'HEAD:frontend/src/App.svelte'], { encoding: 'utf8' });
assert(baselineSource.includes('assetUrls[id] = await asset(id)'));
assert(baselineSource.includes('for (const f of layout.folders)'));
const report = { baseline_commit: baselineCommit, profile: 'same process; controlled 1ms asynchronous asset provider; baseline sequential per-button traversal vs current AssetCache; five samples; no browser/HTTP speed claim', workloads: [] };
for (const count of [48, 1024]) {
  const ids = Array.from({ length: count }, (_, index) => `icon-${index % 8}`);
  const samples = [];
  for (let sample = 0; sample < 5; sample++) {
    let requests = 0, active = 0, peak = 0;
    const asset = async id => {
      requests++; active++; peak = Math.max(peak, active);
      await new Promise(resolve => setTimeout(resolve, 1));
      active--; return `blob:${id}:${requests}`;
    };
    const baselineStart = performance.now();
    const urls = {};
    for (const id of ids) urls[id] = await asset(id);
    const baselineMs = performance.now() - baselineStart;
    assert.equal(requests, count);
    assert.equal(peak, 1);
    requests = 0; peak = 0;
    const cache = new AssetCache(asset, () => {});
    const currentStart = performance.now();
    const loaded = await cache.load(ids);
    const currentMs = performance.now() - currentStart;
    assert.equal(Object.keys(loaded.urls).length, 8);
    assert.equal(requests, 8);
    assert.equal(peak, 4);
    const retainedStart = performance.now();
    await cache.load(ids);
    const retainedMs = performance.now() - retainedStart;
    assert.equal(requests, 8);
    cache.dispose();
    samples.push({ baseline_ms: baselineMs, current_ms: currentMs, retained_ms: retainedMs, baseline_requests: count, current_requests: 8, retained_requests: 0, current_peak_concurrency: peak });
  }
  report.workloads.push({ buttons: count, unique_assets: 8, samples });
}
if (process.env.WEBDECK_ASSET_OUTPUT) writeFileSync(process.env.WEBDECK_ASSET_OUTPUT, JSON.stringify(report, null, 2) + '\n');
console.log('Asset comparison passed: bounded parallel loading, deduplicated requests and retained URL reuse.');
