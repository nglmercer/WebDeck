import type english from './en_US';

type Strings<T> = { [K in keyof T]: T[K] extends string ? string : Strings<T[K]> };
/** Fully translated locales must keep pace with every canonical message. */
export type CompleteLocale = Strings<typeof english>;

type PartialStrings<T> = {
  [K in keyof T]?: T[K] extends string ? string : PartialStrings<T[K]>;
};

/** Partial runtime dictionaries can omit leaves; shipped catalogues use CompleteLocale. */
export type Locale = PartialStrings<Omit<typeof english, 'meta'>> & {
  meta: { [K in keyof typeof english.meta]: string };
};

export type LeafKeys<T> = {
  [K in keyof T & string]: NonNullable<T[K]> extends string
    ? K
    : `${K}.${LeafKeys<NonNullable<T[K]>>}`;
}[keyof T & string];
export type MessageKey = LeafKeys<typeof english>;

const segment = /^[a-zA-Z][a-zA-Z0-9]*$/;
const unsafe = new Set(['__proto__', 'constructor', 'prototype']);

/** The same leaf serializer supplies the frontend fallback and Rust's JSON dictionaries. */
export function flattenMessages<T extends object>(groups: T): Record<LeafKeys<T>, string> {
  const result: Record<string, string> = Object.create(null);
  function visit(node: unknown, prefix = '') {
    if (node === null || typeof node !== 'object' || Array.isArray(node))
      throw new Error(`${prefix || 'catalog'}: expected a message group`);
    const prototype = Object.getPrototypeOf(node);
    if (prototype !== Object.prototype && prototype !== null)
      throw new Error(`${prefix || 'catalog'}: expected a plain message group`);
    for (const [key, value] of Object.entries(node)) {
      if (!segment.test(key) || unsafe.has(key)) throw new Error(`Invalid message segment: ${key}`);
      const name = prefix ? `${prefix}.${key}` : key;
      if (typeof value === 'string') {
        if (!value.trim()) throw new Error(`${name}: expected non-empty text`);
        result[name] = value;
      } else visit(value, name);
    }
  }
  visit(groups);
  return result as Record<LeafKeys<T>, string>;
}
