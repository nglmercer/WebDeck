import { describe, expect, it } from 'vitest';
import { matchingIcons, registeredIcon, iconKey } from './icons';

describe('shared icon registry', () => {
  it('resolves named icons and preserves legacy aliases', () => {
    expect(registeredIcon('icon:play')).toEqual(registeredIcon('▶'));
    expect(registeredIcon('icon:folder')?.fill).toBe('#ffbf00');
    expect(registeredIcon('previous')).not.toEqual(registeredIcon('next'));
    expect(registeredIcon('mute')).not.toEqual(registeredIcon('grid'));
  });
  it('searches registered names and does not silently map unknown names to grid', () => {
    expect(matchingIcons(' CAMERA ')).toEqual(['camera']);
    expect(registeredIcon('icon:unknown')).toBeUndefined();
    expect(registeredIcon('toString')).toBeUndefined();
    expect(iconKey('toString')).toBe('toString');
  });
});
