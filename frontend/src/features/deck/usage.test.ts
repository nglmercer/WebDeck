import { afterEach, expect, it, vi } from 'vitest';
import { flushSync } from 'svelte';
import { request } from '../../lib/api/client';
import { monitorHarness } from './usage.test-harness.svelte';
import type { UsageResponse } from '../../lib/contracts';
vi.mock('../../lib/api/client', () => ({ request: vi.fn() }));
afterEach(() => {
  vi.useRealTimers();
  vi.restoreAllMocks();
});
it('responses do not recreate the polling effect and disposal stops requests', async () => {
  vi.useFakeTimers();
  vi.spyOn(document, 'hidden', 'get').mockReturnValue(false);
  const completions: ((value: UsageResponse) => void)[] = [];
  vi.mocked(request).mockImplementation(
    () =>
      new Promise((resolve) => {
        completions.push(resolve as (value: UsageResponse) => void);
      }),
  );
  const { monitor, dispose } = monitorHarness([
    {
      id: 'usage',
      label: 'Usage',
      icon: '',
      color: '',
      extensions: {},
      action: { type: 'usage' },
    },
  ]);
  try {
    flushSync();
    expect(request).toHaveBeenCalledTimes(1);
    completions[0]!({
      api_version: 2,
      usage: { cpu_percent: 1, cpus: [], memory_used: 1, memory_total: 2, disks: [], gpus: [] },
    });
    await Promise.resolve();
    await Promise.resolve();
    flushSync();
    expect(monitor.status).toBe('live');
    expect(request).toHaveBeenCalledTimes(1);
    await vi.advanceTimersByTimeAsync(1000);
    expect(request).toHaveBeenCalledTimes(2);
    await vi.advanceTimersByTimeAsync(1000);
    expect(request).toHaveBeenCalledTimes(2); // Pending reads never overlap.
  } finally {
    dispose();
  }
  await vi.advanceTimersByTimeAsync(2000);
  expect(request).toHaveBeenCalledTimes(2);
});
