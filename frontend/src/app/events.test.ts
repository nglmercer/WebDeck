import { describe, expect, it, vi } from 'vitest';
import { emitAppEvent, onAppEvent } from './events';

describe('app events', () => {
  it('delivers typed detail to subscribers', () => {
    const seen: Array<{ mode: number }> = [];
    const off = onAppEvent('editor:changed', (detail) => seen.push(detail));
    emitAppEvent('editor:changed', { mode: 1 });
    emitAppEvent('editor:changed', { mode: 0 });
    off();
    emitAppEvent('editor:changed', { mode: 1 });
    expect(seen).toEqual([{ mode: 1 }, { mode: 0 }]);
  });

  it('is observable as window CustomEvents without importing the bus', () => {
    const listener = vi.fn();
    window.addEventListener('webdeck:save:completed', listener);
    emitAppEvent('save:completed', { flow: 'add' });
    window.removeEventListener('webdeck:save:completed', listener);
    expect(listener).toHaveBeenCalledTimes(1);
    expect((listener.mock.calls[0]?.[0] as CustomEvent).detail).toEqual({ flow: 'add' });
  });
});
