import { describe, expect, it } from 'vitest';
import { parseHeaderRows, serializeHeaderRows } from './headers';

describe('parseHeaderRows', () => {
  it('parses Name: value lines, skipping blanks', () => {
    expect(parseHeaderRows('X-Token: abc\n\nContent-Type: application/json\n')).toEqual([
      { name: 'X-Token', value: 'abc' },
      { name: 'Content-Type', value: 'application/json' },
    ]);
  });

  it('splits on the first colon only', () => {
    expect(parseHeaderRows('X-When: 10:30')).toEqual([{ name: 'X-When', value: '10:30' }]);
  });

  it('keeps colon-less lines as valueless names', () => {
    expect(parseHeaderRows('oops')).toEqual([{ name: 'oops', value: '' }]);
  });

  it('parses empty text to no rows', () => {
    expect(parseHeaderRows('')).toEqual([]);
  });
});

describe('serializeHeaderRows', () => {
  it('joins rows with blank names skipped', () => {
    expect(
      serializeHeaderRows([
        { name: 'A', value: '1' },
        { name: '  ', value: 'x' },
        { name: 'B', value: '2' },
      ])
    ).toBe('A: 1\nB: 2');
  });

  it('serializes blank values bare for round-tripping', () => {
    expect(serializeHeaderRows([{ name: 'oops', value: '' }])).toBe('oops');
  });

  it('round-trips parse output', () => {
    const text = 'X-Token: abc\nContent-Type: application/json';
    expect(serializeHeaderRows(parseHeaderRows(text))).toBe(text);
  });
});
