// Static utilities (jQuery.each/map/extend/contains, typed).

/** Iterate an array (`false` breaks). Returns the input. */
export function each<T>(list: ArrayLike<T>, fn: (index: number, value: T) => void | false): ArrayLike<T>;
/** Iterate a record's own entries (`false` breaks). Returns the input. */
export function each<T>(record: Record<string, T>, fn: (key: string, value: T) => void | false): Record<string, T>;
export function each<T>(
  input: ArrayLike<T> | Record<string, T>,
  fn: ((index: number, value: T) => void | false) | ((key: string, value: T) => void | false)
): ArrayLike<T> | Record<string, T> {
  if (typeof (input as ArrayLike<T>).length === 'number') {
    const list = input as ArrayLike<T>;
    const visit = fn as (index: number, value: T) => void | false;
    for (let index = 0; index < list.length; index++) {
      const value = list[index] as T;
      if (visit(index, value) === false) break;
    }
    return input;
  }
  const record = input as Record<string, T>;
  const visit = fn as (key: string, value: T) => void | false;
  for (const [key, value] of Object.entries(record)) {
    if (visit(key, value) === false) break;
  }
  return input;
}

/** Map an array. DEVIATION: results are NOT flattened (jQuery flattens). */
export function map<T, U>(list: ArrayLike<T>, fn: (value: T, index: number) => U): U[];
/** Map a record's values. */
export function map<T, U>(record: Record<string, T>, fn: (value: T, key: string) => U): U[];
export function map<T, U>(
  input: ArrayLike<T> | Record<string, T>,
  fn: ((value: T, index: number) => U) | ((value: T, key: string) => U)
): U[] {
  if (typeof (input as ArrayLike<T>).length === 'number') {
    const list = input as ArrayLike<T>;
    const visit = fn as (value: T, index: number) => U;
    const out: U[] = [];
    for (let index = 0; index < list.length; index++) {
      out.push(visit(list[index] as T, index));
    }
    return out;
  }
  const record = input as Record<string, T>;
  const visit = fn as (value: T, key: string) => U;
  return Object.entries(record).map(([key, value]) => visit(value, key));
}

const UNSAFE_KEYS = new Set(['__proto__', 'constructor', 'prototype']);

function isPlainRecord(value: unknown): value is Record<string, unknown> {
  if (typeof value !== 'object' || value === null) return false;
  const proto: unknown = Object.getPrototypeOf(value);
  return proto === Object.prototype || proto === null;
}

/** Deep-merge one value over the current one (arrays merge by index). */
function deepMergeValue(current: unknown, value: unknown): unknown {
  if (Array.isArray(value)) {
    const base: unknown[] = Array.isArray(current) ? [...current] : [];
    value.forEach((item, index) => {
      if (item === undefined) return;
      base[index] =
        isPlainRecord(item) || Array.isArray(item) ? deepMergeValue(base[index], item) : item;
    });
    return base;
  }
  if (isPlainRecord(value)) {
    const base: Record<string, unknown> = isPlainRecord(current) ? { ...current } : {};
    for (const [key, item] of Object.entries(value)) {
      // DEVIATION (hardening): never merge prototype-polluting keys.
      if (UNSAFE_KEYS.has(key)) continue;
      if (item === undefined) continue;
      base[key] = isPlainRecord(item) || Array.isArray(item) ? deepMergeValue(base[key], item) : item;
    }
    return base;
  }
  return value;
}

function mergeInto(deep: boolean, target: Record<string, unknown>, source: unknown): void {
  if (!isPlainRecord(source)) return;
  for (const [key, value] of Object.entries(source)) {
    // DEVIATION (hardening): never merge prototype-polluting keys.
    if (UNSAFE_KEYS.has(key)) continue;
    if (value === undefined) continue;
    target[key] =
      deep && (isPlainRecord(value) || Array.isArray(value))
        ? deepMergeValue(target[key], value)
        : value;
  }
}

/**
 * Merge sources into target (mutates and returns it). `deep` clones
 * nested plain objects/arrays instead of sharing them.
 */
export function extend<T extends object>(target: T, ...sources: unknown[]): T;
export function extend<T extends object>(deep: boolean, target: T, ...sources: unknown[]): T;
export function extend<T extends object>(
  targetOrDeep: T | boolean,
  ...rest: unknown[]
): T {
  const deep = typeof targetOrDeep === 'boolean' ? targetOrDeep : false;
  const target = (typeof targetOrDeep === 'boolean' ? rest.shift() : targetOrDeep) as T;
  const record = target as unknown as Record<string, unknown>;
  for (const source of rest) {
    mergeInto(deep, record, source);
  }
  return target;
}

/** True when `parent` strictly contains `child`. */
export function contains(parent: Element, child: Element): boolean {
  return parent !== child && parent.contains(child);
}
