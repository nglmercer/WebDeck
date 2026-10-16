import { describe, it, expect } from 'vitest';
import { normalizeKey, validateChord, captureChord, keys } from './keys';
describe('canonical keyboard chords', () => {
  it('preserves aliases and all function keys', () => {
    expect(normalizeKey('Control')).toBe('ctrl');
    for (let n = 1; n <= 24; n++) expect(keys.some((k) => k.id === `f${n}`)).toBe(true);
    expect(validateChord(['Control', 'c'])).toBeUndefined();
  });
  it('rejects unknown, firmware and duplicate modifiers', () => {
    for (const chord of [[], ['fn'], ['ctrl', 'Control'], ['wat']])
      expect(validateChord(chord)).toBeTruthy();
  });
  it('captures a focused logical chord and ignores modifier and numpad ambiguity', () => {
    expect(
      captureChord(
        new KeyboardEvent('keydown', { key: 'F12', code: 'F12', ctrlKey: true, shiftKey: true }),
      ),
    ).toEqual(['ctrl', 'shift', 'f12']);
    expect(
      captureChord(new KeyboardEvent('keydown', { key: 'Control', code: 'ControlLeft' })),
    ).toBeUndefined();
    expect(captureChord(new KeyboardEvent('keydown', { key: '1', code: 'Numpad1' }))).toEqual([
      'numpad1',
    ]);
  });
});
