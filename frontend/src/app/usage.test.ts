import { afterEach, describe, expect, it, vi } from 'vitest';
import { pollUsageOnce, startUsageLoop, stopUsageLoop } from './usage';

describe('usage loop', () => {
  afterEach(() => {
    stopUsageLoop();
    vi.useRealTimers();
    vi.unstubAllGlobals();
    document.body.innerHTML = '';
  });

  it('polls immediately on start instead of waiting a full interval', () => {
    const fetchMock = vi.fn(async (_url: string) => ({ ok: true, json: async () => ({}) }));
    vi.stubGlobal('fetch', fetchMock);
    vi.useFakeTimers();
    startUsageLoop(3000);
    // No timers advanced: the first poll must already be in flight.
    expect(fetchMock.mock.calls.filter((call) => String(call[0]).includes('/usage'))).toHaveLength(
      1
    );
  });

  it('pollUsageOnce tolerates missing tiles', () => {
    const fetchMock = vi.fn(async () => ({ ok: true, json: async () => ({}) }));
    vi.stubGlobal('fetch', fetchMock);
    document.body.innerHTML = '<div id="app"></div>';
    expect(() => pollUsageOnce()).not.toThrow();
  });
});
