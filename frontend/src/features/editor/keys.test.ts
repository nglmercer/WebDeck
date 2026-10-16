import { describe, expect, it } from 'vitest';
import {
  captureChord,
  hostStatusMessage,
  keyStatusMessage,
  keyUnavailable,
  keys,
  normalizeKey,
  platformSupport,
  selectPrimaryKey,
  toggleModifier,
  validateChord,
  validatePickerChord,
  validatePickerChordForHost,
} from './keys';

describe('canonical keyboard chords', () => {
  it('normalizes aliases without changing legacy values until they are edited', () => {
    expect(normalizeKey('Control')).toBe('ctrl');
    expect(validateChord(['Control', 'c'])).toBeUndefined();
    expect(validateChord(['Ctrl', 'Control'])).toBe('Remove duplicate keys or modifier aliases.');
  });

  it('keeps every function key discoverable and the Fn key unsupported', () => {
    for (let number = 1; number <= 24; number++)
      expect(keys.some((key) => key.id === `f${number}`)).toBe(true);
    expect(validateChord(['fn'])).toContain('unavailable');
    expect(keyUnavailable('fn')).toBe(true);
  });

  it('separates catalog validation from host mapping and permission state', () => {
    expect(platformSupport('f13')).toBe('unknown');
    expect(platformSupport('f13', 'windows')).toBe('supported');
    expect(hostStatusMessage()).toBe('ui_keypicker_host_unknown');
    expect(keyStatusMessage('f24')).toBe('ui_keypicker_host_unknown');
    expect(keyUnavailable('f24')).toBe(false);
    expect(
      hostStatusMessage({
        catalog_version: 1,
        platform: 'linux_wayland',
        availability: 'portal_approval_required',
        fn_status: 'unsupported',
      }),
    ).toBe('ui_keypicker_permission_required');
    expect(validatePickerChordForHost(['f24'], false)).toBeUndefined();
    expect(
      validatePickerChordForHost(['f24'], false, {
        catalog_version: 1,
        platform: 'unsupported',
        availability: 'unsupported',
        fn_status: 'unsupported',
      }),
    ).toBe('Keyboard input is unsupported on this host.');
  });

  it('uses one primary key by default and preserves multiple keys in advanced mode', () => {
    expect(validatePickerChord(['ctrl'], false)).toContain('primary key');
    expect(validatePickerChord(['ctrl', 'f1', 'c'], false)).toContain('advanced');
    expect(validatePickerChord(['ctrl', 'f1', 'c'], true)).toBeUndefined();
    expect(validatePickerChord(['ctrl', 'shift', 'alt', 'meta', 'f1', 'a'], true)).toBeUndefined();
    expect(
      validatePickerChord(
        Array.from({ length: 17 }, (_, index) => String(index)),
        true,
      ),
    ).toBe('Choose between 1 and 16 keys.');
  });

  it('toggles modifiers and replaces only the primary key in normal mode', () => {
    expect(toggleModifier(['Control', 'c'], 'shift')).toEqual(['ctrl', 'shift', 'c']);
    expect(toggleModifier(['Control', 'c'], 'ctrl')).toEqual(['c']);
    expect(selectPrimaryKey(['ctrl', 'c'], 'f24', false)).toEqual(['ctrl', 'f24']);
    expect(selectPrimaryKey(['ctrl', 'f1', 'c'], 'f24', true)).toEqual(['ctrl', 'f1', 'c', 'f24']);
  });

  it('captures logical keys while ignoring modifiers, repeats, IME and unsupported numpad variants', () => {
    expect(
      captureChord(
        new KeyboardEvent('keydown', {
          key: 'F12',
          code: 'F12',
          ctrlKey: true,
          shiftKey: true,
        }),
      ),
    ).toEqual(['ctrl', 'shift', 'f12']);
    expect(
      captureChord(new KeyboardEvent('keydown', { key: 'Control', code: 'ControlLeft' })),
    ).toBe(undefined);
    expect(captureChord(new KeyboardEvent('keydown', { key: 'AltGraph', code: 'AltRight' }))).toBe(
      undefined,
    );
    expect(
      captureChord(new KeyboardEvent('keydown', { key: 'x', code: 'KeyX', repeat: true })),
    ).toBeUndefined();
    expect(
      captureChord(new KeyboardEvent('keydown', { key: 'x', code: 'KeyX', isComposing: true })),
    ).toBeUndefined();
    expect(
      captureChord(new KeyboardEvent('keydown', { key: '+', code: 'NumpadAdd' })),
    ).toBeUndefined();
    expect(captureChord(new KeyboardEvent('keydown', { key: '1', code: 'Numpad1' }))).toEqual([
      'numpad1',
    ]);
  });
});
