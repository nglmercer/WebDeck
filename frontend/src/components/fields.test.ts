import { describe, expect, it } from 'vitest';
import { normalizeHexColor } from './fields';

describe('normalizeHexColor', () => {
  it('prefixes a missing hash and trims', () => {
    expect(normalizeHexColor('  ff0000 ')).toBe('#ff0000');
    expect(normalizeHexColor('#00ff00')).toBe('#00ff00');
    expect(normalizeHexColor('')).toBe('');
    expect(normalizeHexColor('   ')).toBe('');
  });
});
