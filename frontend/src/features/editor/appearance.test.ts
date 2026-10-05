import { it, expect } from 'vitest';
import { appearanceOf, setAppearance, tileColors } from './appearance';
it('normalizes malformed appearance without discarding unknown extension keys', () => {
  const owner = { extensions: { appearance: { gap: 8, custom: { retained: true } }, vendor: 42 } };
  setAppearance(owner, 'gap', 12);
  expect(owner.extensions).toEqual({
    appearance: { gap: 12, custom: { retained: true } },
    vendor: 42,
  });
  expect(appearanceOf({ extensions: { appearance: 'invalid' } })).toEqual({});
  expect(() => setAppearance(owner, 'gap', NaN)).toThrow();
  expect(() => setAppearance(owner, 'columns', 1.5)).toThrow();
});
it('chooses readable text for dark and bright custom colors', () => {
  expect(tileColors('#111111').foreground).toBe('#ffffff');
  expect(tileColors('#ffffff').foreground).toBe('#000000');
  for (const color of ['#8276e9', '#ff0000', '#000', '#ffff00', '#00ff00'])
    expect(tileColors(color).contrast).toBeGreaterThanOrEqual(4.5);
});
