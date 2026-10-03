import { describe, it, expect, vi } from 'vitest';
import { contract, defaultValue, commandSchema, execute } from './api';
import config from '../../webdeck/config_default.json';
describe('v2 contracts', () => {
  it('rejects the old command shape, unknown fields and invalid bounds', () => {
    for (const v of [
      { message: '/key x' },
      { request_id: 'id', command: { type: 'volume', change: { type: 'set', percent: 101 } } },
      { request_id: 'id', command: { type: 'write', text: 'hello', send: 'false' } },
      { request_id: 'id', command: { type: 'play_pause', extra: 1 } },
    ])
      expect(() => contract('CommandRequest', v)).toThrow();
  });
  it('keeps booleans, arrays, nested sources and extensions typed', () => {
    expect(contract('Config', config)).toEqual(config);
    expect(
      contract('CommandRequest', {
        request_id: 'abc',
        command: { type: 'script', source: { type: 'inline', code: '42' } },
      }),
    ).toBeTruthy();
    expect(defaultValue(commandSchema)).toEqual({ type: 'debug', data: {} });
  });
  it('validates correlation transitions and native-picker cancellation', () => {
    expect(
      contract('CommandEvent', { api_version: 2, request_id: 'abc', state: 'accepted' }),
    ).toBeTruthy();
    expect(contract('SelectionResponse', { api_version: 2, source: null })).toBeTruthy();
    expect(() =>
      contract('CommandEvent', { api_version: 2, request_id: 'abc', state: 'completed' }),
    ).toThrow();
  });
});

it('reports an HTTP timeout without replaying a native command', async () => {
  const fetch = vi.fn().mockRejectedValue(new DOMException('Timed out', 'TimeoutError'));
  vi.stubGlobal('fetch', fetch);
  try {
    await expect(execute({ type: 'play_pause' }, 'http')).rejects.toThrow(
      'execution outcome is unknown',
    );
    expect(fetch).toHaveBeenCalledTimes(1);
    expect(fetch.mock.calls[0]?.[1]?.signal).toBeInstanceOf(AbortSignal);
  } finally {
    vi.unstubAllGlobals();
  }
});
