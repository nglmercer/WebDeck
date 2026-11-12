import { describe, expect, it } from 'vitest';
import { browserDeviceName, suggestDeviceName } from './device-name';

describe('pairing device name suggestions', () => {
  it('recognizes iPhone, touch iPad desktop mode, and reduced Android agents', () => {
    expect(
      browserDeviceName({
        userAgent: 'Mozilla/5.0 (iPhone; CPU iPhone OS 18_0) AppleWebKit Safari/605.1',
      }),
    ).toBe('Safari · iPhone');
    expect(
      browserDeviceName({
        userAgent: 'Mozilla/5.0 (Macintosh) AppleWebKit Safari/605.1',
        platform: 'MacIntel',
        maxTouchPoints: 5,
      }),
    ).toBe('Safari · iPad');
    expect(
      browserDeviceName({
        userAgent: 'Mozilla/5.0 (Linux; Android 10; K) Chrome/140.0 Mobile Safari/537.36',
      }),
    ).toBe('Chrome · Android');
    expect(
      browserDeviceName({
        userAgent: 'Mozilla/5.0 (Windows NT 10.0) Chrome/140.0 Safari/537.36 Edg/140.0',
      }),
    ).toBe('Edge · Windows');
  });
  it('uses an available model and tolerates missing or denied hints', async () => {
    const base = { userAgent: 'Android Chrome/140.0' };
    expect(
      await suggestDeviceName(
        { ...base, userAgentData: { getHighEntropyValues: async () => ({ model: 'Pixel 9' }) } },
        'My phone',
      ),
    ).toBe('Pixel 9');
    expect(
      await suggestDeviceName(
        {
          ...base,
          userAgentData: {
            getHighEntropyValues: async () => {
              throw Error('denied');
            },
          },
        },
        'My phone',
      ),
    ).toBe('Chrome · Android');
    expect(
      await suggestDeviceName(
        { ...base, userAgentData: { getHighEntropyValues: async () => ({ model: 'K' }) } },
        'My phone',
      ),
    ).toBe('Chrome · Android');
    expect(await suggestDeviceName({ userAgent: '' }, 'Mi teléfono')).toBe('Mi teléfono');
  });
  it('bounds the wait so an unavailable hints service cannot block pairing', async () => {
    expect(
      await suggestDeviceName(
        {
          userAgent: 'Android Chrome/140.0',
          userAgentData: { getHighEntropyValues: () => new Promise(() => {}) },
        },
        'My phone',
      ),
    ).toBe('Chrome · Android');
  });
});
