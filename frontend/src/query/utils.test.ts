import { describe, expect, it } from 'vitest';
import { contains, each, extend, map, q } from './index';

describe('utils', () => {
  it('each() iterates arrays and records, false breaks', () => {
    const seen: Array<string | number> = [];
    const arr = each(['a', 'b'], (index, value) => {
      seen.push(`${index}:${value}`);
    });
    expect(seen).toEqual(['0:a', '1:b']);
    expect(arr).toEqual(['a', 'b']);
    const keys: string[] = [];
    each({ x: 1, y: 2 }, (key) => {
      keys.push(key);
      return false;
    });
    expect(keys).toEqual(['x']);
  });

  it('map() transforms without flattening', () => {
    expect(map([1, 2], (v, i) => v + i)).toEqual([1, 3]);
    expect(map({ a: 1 }, (v, k) => `${k}${v}`)).toEqual(['a1']);
    expect(map([1], (v) => [v])).toEqual([[1]]);
  });

  it('extend() merges shallow and deep', () => {
    expect(extend({ a: 1 }, { b: 2 }, { a: 3 })).toEqual({ a: 3, b: 2 });
    const target = { nested: { keep: 1, list: [1] } };
    const source = { nested: { list: [2], add: 9 } };
    const merged = extend(true, target, source);
    expect(merged).toBe(target);
    expect(merged.nested).toEqual({ keep: 1, list: [2], add: 9 });
    // Deep merge clones: mutating the result leaves the source alone.
    merged.nested.list.push(3);
    expect(source.nested.list).toEqual([2]);
  });

  it('extend() refuses prototype pollution', () => {
    const target: Record<string, unknown> = {};
    extend(true, target, JSON.parse('{"__proto__": {"polluted": true}}'));
    extend(target, { constructor: 1 }, { prototype: 2 });
    expect(({} as Record<string, unknown>)['polluted']).toBeUndefined();
    expect(target['__proto__']).not.toEqual({ polluted: true });
    expect(target['constructor']).not.toBe(1);
    expect(Object.prototype.hasOwnProperty.call(target, 'constructor')).toBe(false);
  });

  it('contains() is strict', () => {
    document.body.innerHTML = '<div id="p"><span id="c"></span></div>';
    const parent = document.getElementById('p');
    const child = document.getElementById('c');
    expect(parent && child ? contains(parent, child) : false).toBe(true);
    expect(parent && child ? contains(child, parent) : true).toBe(false);
    expect(parent ? contains(parent, parent) : true).toBe(false);
    expect(q('#p').length).toBe(1);
  });
});
