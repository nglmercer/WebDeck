import { describe, it, expect } from 'vitest';
import { contract, defaultValue, commandSchema } from './api';
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
