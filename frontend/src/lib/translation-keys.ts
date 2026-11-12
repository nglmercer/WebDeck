import type english from '../../locales/en_US';
import { messages } from './translations';

type KeyTree<T, Prefix extends string = ''> = {
  readonly [K in keyof T & string]: T[K] extends string
    ? `${Prefix}${K}`
    : KeyTree<T[K], `${Prefix}${K}.`>;
};

function buildKeys() {
  const tree: Record<string, unknown> = Object.create(null);
  for (const key of Object.keys(messages)) {
    const parts = key.split('.');
    let group = tree;
    for (const part of parts.slice(0, -1)) {
      group[part] ??= Object.create(null);
      group = group[part] as Record<string, unknown>;
    }
    group[parts.at(-1)!] = key;
  }
  return tree as KeyTree<typeof english>;
}

/** Typed paths share the fallback's keys; no second copy of the catalogue is bundled. */
export const keys = buildKeys();
export const common = keys.common;
