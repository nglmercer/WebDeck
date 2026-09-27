// Q data store + data methods (extracted from core.ts).
//
// This module owns `dataStore`: entries die with their element (WeakMap),
// and the purge/clone helpers keep manipulate.ts consistent.

import { Q } from './core';
import { purgeListenersDeep } from './events';

declare module './core' {
  interface Q<T extends Element> {
    /** Stored data (or coerced `data-*` attribute) for `key`. */
    data<U = unknown>(key: string): U | undefined;
    /** All data including `data-*` attributes (camelCase keys). */
    data(): Record<string, unknown>;
    /** Store one value. */
    data(key: string, value: unknown): this;
    /** Store several values. */
    data(values: Record<string, unknown>): this;
    /** Drop stored data (`key` omitted clears everything for these elements). */
    removeData(key?: string | string[]): this;
  }
}

/** Per-element data store (WeakMap: entries die with their element). */
const dataStore = new WeakMap<Element, Map<string, unknown>>();

/** Coerce a `data-*` attribute string the way jQuery does. */
function coerceDataValue(raw: string): unknown {
  if (raw === 'true') return true;
  if (raw === 'false') return false;
  if (raw === 'null') return null;
  if (raw !== '' && !Number.isNaN(Number(raw))) return Number(raw);
  const first = raw.charAt(0);
  if (first === '{' || first === '[') {
    try {
      return JSON.parse(raw) as unknown;
    } catch {
      return raw;
    }
  }
  return raw;
}

function readDataAttribute(element: Element, key: string): unknown {
  // jQuery maps camelCase keys to dash-case attributes and back.
  const dashed = key.replace(/[A-Z]/g, (ch) => `-${ch.toLowerCase()}`);
  const attr =
    element.getAttribute(`data-${dashed}`) ?? (dashed === key ? null : element.getAttribute(`data-${key}`));
  return attr === null ? undefined : coerceDataValue(attr);
}

/** Remove our data entries for an element subtree (jQuery.cleanData). */
export function purgeData(root: Element): void {
  dataStore.delete(root);
  for (const child of root.querySelectorAll('*')) {
    dataStore.delete(child);
  }
}

/** Copy one element's data entries. */
export function cloneData(src: Element, dst: Element): void {
  const map = dataStore.get(src);
  if (map) dataStore.set(dst, new Map(map));
}

/** Purge data + listeners of an element's children (keeps its own). */
export function purgeChildren(el: Element): void {
  for (const child of [...el.children]) {
    purgeData(child);
    purgeListenersDeep(child);
  }
}

/** Copy data entries pairwise across two parallel subtrees. */
export function cloneDataDeep(src: Element, dst: Element): void {
  cloneData(src, dst);
  const srcKids = [...src.querySelectorAll('*')];
  const dstKids = [...dst.querySelectorAll('*')];
  srcKids.forEach((kid, index) => {
    const twin = dstKids[index];
    if (twin !== undefined) cloneData(kid, twin);
  });
}

function dataImpl(
  this: Q,
  keyOrValues?: string | Record<string, unknown>,
  value?: unknown
): unknown {
  if (keyOrValues === undefined) {
    const first = this.els[0];
    if (!first) return {};
    const out: Record<string, unknown> = {};
    for (const attr of first.attributes) {
      if (attr.name.startsWith('data-')) {
        const key = attr.name
          .slice(5)
          .replace(/-([a-z])/g, (_, ch: string) => ch.toUpperCase());
        out[key] = coerceDataValue(attr.value);
      }
    }
    const stored = dataStore.get(first);
    if (stored) {
      for (const [k, v] of stored) out[k] = v;
    }
    return out;
  }
  if (typeof keyOrValues === 'string' && value === undefined) {
    const first = this.els[0];
    if (!first) return undefined;
    const stored = dataStore.get(first)?.get(keyOrValues);
    if (stored !== undefined) return stored;
    return readDataAttribute(first, keyOrValues);
  }
  const entries: Array<[string, unknown]> =
    typeof keyOrValues === 'string' ? [[keyOrValues, value]] : Object.entries(keyOrValues);
  for (const el of this.els) {
    let map = dataStore.get(el);
    if (!map) {
      map = new Map();
      dataStore.set(el, map);
    }
    for (const [k, v] of entries) map.set(k, v);
  }
  return this;
}

Q.prototype.data = dataImpl as Q<Element>['data'];

Q.prototype.removeData = function <T extends Element>(this: Q<T>, key?: string | string[]): Q<T> {
  for (const el of this.els) {
    if (key === undefined) {
      dataStore.delete(el);
      continue;
    }
    const map = dataStore.get(el);
    if (!map) continue;
    for (const k of Array.isArray(key) ? key : [key]) map.delete(k);
  }
  return this;
};
