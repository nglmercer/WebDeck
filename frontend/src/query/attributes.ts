// Q attribute/property/value methods (extracted from core.ts).

import { Q } from './core';

declare module './core' {
  interface Q<T extends Element> {
    /** Attribute value of the first element (`undefined` when missing). */
    attr(name: string): string | undefined;
    /** Set an attribute (`null` removes it). */
    attr(name: string, value: string | number | boolean | null): this;
    /** Set several attributes at once. */
    attr(values: Record<string, string | number | boolean | null>): this;
    /** Remove one or more (space-separated) attributes. */
    removeAttr(names: string): this;
    /** Typed DOM property of the first element. */
    prop<K extends keyof T>(name: K): T[K] | undefined;
    /** Set a typed DOM property on every element. */
    prop<K extends keyof T>(name: K, value: T[K]): this;
    /** Set several properties at once. */
    prop(values: { [K in keyof T]?: T[K] }): this;
    /**
     * Value of the first form element. Multi-selects yield an array,
     * anything else yields `undefined`.
     */
    val(): string | string[] | undefined;
    /** Set the value of input/select/textarea elements. */
    val(value: string | number | string[] | null): this;
  }
}

function attrImpl<T extends Element>(
  this: Q<T>,
  nameOrValues: string | Record<string, string | number | boolean | null>,
  value?: string | number | boolean | null
): string | undefined | Q<T> {
  if (typeof nameOrValues === 'string' && value === undefined) {
    return this.els[0]?.getAttribute(nameOrValues) ?? undefined;
  }
  const entries: Array<[string, string | number | boolean | null]> =
    typeof nameOrValues === 'string' ? [[nameOrValues, value ?? null]] : Object.entries(nameOrValues);
  for (const el of this.els) {
    for (const [name, v] of entries) {
      if (v === null) el.removeAttribute(name);
      else if (typeof v === 'boolean') {
        if (v) el.setAttribute(name, '');
        else el.removeAttribute(name);
      } else el.setAttribute(name, String(v));
    }
  }
  return this;
}

Q.prototype.attr = attrImpl as Q<Element>['attr'];

Q.prototype.removeAttr = function <T extends Element>(this: Q<T>, names: string): Q<T> {
  const list = names.split(/\s+/).filter((n) => n !== '');
  for (const el of this.els) {
    for (const name of list) el.removeAttribute(name);
  }
  return this;
};

function propImpl<T extends Element, K extends keyof T>(
  this: Q<T>,
  nameOrValues: K | { [J in keyof T]?: T[J] },
  value?: T[K]
): T[K] | undefined | Q<T> {
  if ((typeof nameOrValues === 'string' || typeof nameOrValues === 'number') && value === undefined) {
    const first = this.els[0];
    return first?.[nameOrValues];
  }
  if (typeof nameOrValues === 'object') {
    for (const el of this.els) {
      for (const [k, v] of Object.entries(nameOrValues)) {
        (el as unknown as Record<string, unknown>)[k] = v;
      }
    }
    return this;
  }
  for (const el of this.els) {
    el[nameOrValues as K] = value as T[K];
  }
  return this;
}

Q.prototype.prop = propImpl as Q<Element>['prop'];

function valImpl<T extends Element>(
  this: Q<T>,
  value?: string | number | string[] | null
): string | string[] | undefined | Q<T> {
  if (value === undefined) {
    const first = this.els[0];
    if (first instanceof HTMLSelectElement && first.multiple) {
      return [...first.selectedOptions].map((o) => o.value);
    }
    if (first instanceof HTMLInputElement || first instanceof HTMLTextAreaElement || first instanceof HTMLSelectElement) {
      return first.value;
    }
    return undefined;
  }
  const v = value === null ? '' : value;
  for (const el of this.els) {
    if (Array.isArray(v)) {
      if (el instanceof HTMLSelectElement && el.multiple) {
        for (const option of el.options) {
          option.selected = v.includes(option.value);
        }
      } else if (el instanceof HTMLInputElement && (el.type === 'checkbox' || el.type === 'radio')) {
        el.checked = v.includes(el.value);
      }
    } else if (el instanceof HTMLInputElement || el instanceof HTMLTextAreaElement || el instanceof HTMLSelectElement) {
      el.value = String(v);
    }
  }
  return this;
}

Q.prototype.val = valImpl as Q<Element>['val'];
