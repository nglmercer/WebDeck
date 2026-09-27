// qdom — a modern, typesafe jQuery replacement (zero dependencies).
//
// Why it is better than jQuery:
// - Generic element typing: `q('input')` is `Q<HTMLInputElement>`, so
//   `.val()`, `.prop('checked')`, … are checked per element type.
// - Typed events: `on('click', …)` narrows the event object via
//   `HTMLElementEventMap`; delegation keeps `this` typed too.
// - No silent footguns: index access is `T | undefined`, getters on empty
//   sets return `undefined`, and invalid selectors throw real errors.
// - Effects run on the Web Animations API and return Promises (no fx
//   queue, no timer hacks); `prefers-reduced-motion` is honored.
// - Inserted HTML never executes scripts; `extend` cannot pollute
//   prototypes (`__proto__`/`constructor`/`prototype` are skipped).
//
// Deliberate deviations from jQuery are marked "DEVIATION" below.

import {
  addListener,
  cloneListenersDeep,
  purgeListenersDeep,
  removeListener,
  type EventHandler,
} from './events';
import {
  animateOpacity,
  animateShowHide,
  animateSlide,
  hideInstant,
  isHidden,
  runAnimation,
  showInstant,
  stopAnimations,
  type EffectOptions,
} from './effects';

/** Content accepted by insertion methods. */
export type Content = string | Element | Q<Element> | ArrayLike<string | Element>;

/** Predicate form shared by filter/not/is. */
export type ElementPredicate<T extends Element> = (this: T, index: number, element: T) => unknown;

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
function purgeData(root: Element): void {
  dataStore.delete(root);
  for (const child of root.querySelectorAll('*')) {
    dataStore.delete(child);
  }
}

/** Copy one element's data entries. */
function cloneData(src: Element, dst: Element): void {
  const map = dataStore.get(src);
  if (map) dataStore.set(dst, new Map(map));
}

/** Purge data + listeners of an element's children (keeps its own). */
function purgeChildren(el: Element): void {
  for (const child of [...el.children]) {
    purgeData(child);
    purgeListenersDeep(child);
  }
}

/** Copy data entries pairwise across two parallel subtrees. */
function cloneDataDeep(src: Element, dst: Element): void {
  cloneData(src, dst);
  const srcKids = [...src.querySelectorAll('*')];
  const dstKids = [...dst.querySelectorAll('*')];
  srcKids.forEach((kid, index) => {
    const twin = dstKids[index];
    if (twin !== undefined) cloneData(kid, twin);
  });
}

/** Parse an HTML string into nodes (scripts never execute in <template>). */
function parseNodes(html: string): Node[] {
  const template = document.createElement('template');
  template.innerHTML = html.trim();
  return [...template.content.childNodes];
}

/** Normalize insertion content to fresh node lists per target. */
function normalizeContent(content: Content): Node[] {
  if (typeof content === 'string') return parseNodes(content);
  if (content instanceof Q) return content.toArray();
  if (content instanceof Element) return [content];
  const out: Node[] = [];
  for (const item of Array.from(content)) {
    if (typeof item === 'string') out.push(...parseNodes(item));
    else out.push(item);
  }
  return out;
}

/** jQuery scheme: originals go to the LAST target, clones elsewhere. */
function distribute(targets: Element[], nodes: Node[]): void {
  targets.forEach((target, index) => {
    const last = index === targets.length - 1;
    for (const node of nodes) {
      target.appendChild(last ? node : node.cloneNode(true));
    }
  });
}

function matchesSelector(element: Element, selector: string): boolean {
  return element.matches(selector);
}

function unique<T extends Element>(elements: T[]): T[] {
  return [...new Set(elements)];
}

/**
 * Chainable, array-like wrapper around a fixed element list.
 * Empty sets are safe: setters/chainers no-op, getters return `undefined`.
 */
export class Q<T extends Element = Element> {
  private readonly els: readonly T[];

  constructor(elements: ArrayLike<T> | null | undefined) {
    const arr = elements ? Array.from(elements) : [];
    this.els = arr;
    let index = 0;
    for (const el of arr) {
      this[index] = el;
      index++;
    }
  }

  /** Numeric access like jQuery (`q('div')[0]`), honestly `T | undefined`. */
  [index: number]: T;

  get length(): number {
    return this.els.length;
  }

  [Symbol.iterator](): IterableIterator<T> {
    return this.els[Symbol.iterator]();
  }

  /** First element (or index `i`, negative counts from the end). */
  get(index: number): T | undefined;
  /** All elements as a real array. */
  get(): T[];
  get(index?: number): T | undefined | T[] {
    if (index === undefined) return [...this.els];
    const i = index < 0 ? this.els.length + index : index;
    return this.els[i];
  }

  toArray(): T[] {
    return [...this.els];
  }

  /** Iterate; return `false` to break early (jQuery semantics). */
  each(fn: (this: T, index: number, element: T) => void | false): this {
    let index = 0;
    for (const el of this.els) {
      if (fn.call(el, index, el) === false) break;
      index++;
    }
    return this;
  }

  /** Map to an array. DEVIATION: results are NOT flattened (jQuery flattens). */
  map<U>(fn: (this: T, index: number, element: T) => U): U[] {
    const out: U[] = [];
    let index = 0;
    for (const el of this.els) {
      out.push(fn.call(el, index, el));
      index++;
    }
    return out;
  }

  /** Reduce to the element at `index` (negative counts from the end). */
  eq(index: number): Q<T> {
    const i = index < 0 ? this.els.length + index : index;
    const el = this.els[i];
    return new Q(el === undefined ? [] : [el]);
  }

  first(): Q<T> {
    return this.eq(0);
  }

  last(): Q<T> {
    return this.eq(-1);
  }

  slice(start?: number, end?: number): Q<T> {
    return new Q(this.els.slice(start, end));
  }

  /** True when at least one element matches. */
  is(selector: string): boolean;
  is(element: Element): boolean;
  is(set: Q<Element>): boolean;
  is(predicate: ElementPredicate<T>): boolean;
  is(match: string | Element | Q<Element> | ElementPredicate<T>): boolean {
    if (typeof match === 'string') return this.els.some((el) => matchesSelector(el, match));
    if (match instanceof Q) {
      const set = new Set(match.toArray());
      return this.els.some((el) => set.has(el));
    }
    if (match instanceof Element) return this.els.includes(match as T);
    return this.els.some((el, index) => match.call(el, index, el));
  }

  filter(selector: string): Q<T>;
  filter(predicate: ElementPredicate<T>): Q<T>;
  filter(element: Element): Q<T>;
  filter(set: Q<Element>): Q<T>;
  filter(match: string | Element | Q<Element> | ElementPredicate<T>): Q<T> {
    if (typeof match === 'string') return new Q(this.els.filter((el) => matchesSelector(el, match)));
    if (match instanceof Q) {
      const set = new Set(match.toArray());
      return new Q(this.els.filter((el) => set.has(el)));
    }
    if (match instanceof Element) return new Q(this.els.filter((el) => el === match));
    return new Q(this.els.filter((el, index) => match.call(el, index, el)));
  }

  not(selector: string): Q<T>;
  not(predicate: ElementPredicate<T>): Q<T>;
  not(element: Element): Q<T>;
  not(set: Q<Element>): Q<T>;
  not(match: string | Element | Q<Element> | ElementPredicate<T>): Q<T> {
    if (typeof match === 'string') return new Q(this.els.filter((el) => !matchesSelector(el, match)));
    if (match instanceof Q) {
      const set = new Set(match.toArray());
      return new Q(this.els.filter((el) => !set.has(el)));
    }
    if (match instanceof Element) return new Q(this.els.filter((el) => el !== match));
    return new Q(this.els.filter((el, index) => !match.call(el, index, el)));
  }

  /** Keep elements that contain a descendant matching `selector`/`element`. */
  has(selector: string): Q<T>;
  has(element: Element): Q<T>;
  has(match: string | Element): Q<T> {
    if (typeof match === 'string') {
      return new Q(this.els.filter((el) => el.querySelector(match) !== null));
    }
    return new Q(this.els.filter((el) => el !== match && el.contains(match)));
  }

  /** Descendants matching `selector` (deduped, document order). */
  find(selector: string): Q<Element> {
    const out: Element[] = [];
    for (const el of this.els) {
      out.push(...el.querySelectorAll(selector));
    }
    return new Q(unique(out));
  }

  /** Direct child elements, optionally filtered. */
  children(selector?: string): Q<Element> {
    const out: Element[] = [];
    for (const el of this.els) {
      for (const child of el.children) {
        if (selector === undefined || matchesSelector(child, selector)) out.push(child);
      }
    }
    return new Q(out);
  }

  /** Unique immediate parents, optionally filtered. */
  parent(selector?: string): Q<Element> {
    const out: Element[] = [];
    for (const el of this.els) {
      const p = el.parentElement;
      if (p !== null && (selector === undefined || matchesSelector(p, selector))) out.push(p);
    }
    return new Q(unique(out));
  }

  /** All ancestors (nearest first), optionally filtered. */
  parents(selector?: string): Q<Element> {
    const out: Element[] = [];
    for (const el of this.els) {
      let p = el.parentElement;
      while (p !== null) {
        if (selector === undefined || matchesSelector(p, selector)) out.push(p);
        p = p.parentElement;
      }
    }
    return new Q(unique(out));
  }

  /** First ancestor-or-self matching `selector`. */
  closest(selector: string): Q<Element> {
    const out: Element[] = [];
    for (const el of this.els) {
      const found = el.closest(selector);
      if (found !== null) out.push(found);
    }
    return new Q(unique(out));
  }

  /** Sibling elements (excluding self), optionally filtered. */
  siblings(selector?: string): Q<Element> {
    const out: Element[] = [];
    for (const el of this.els) {
      const parent = el.parentElement;
      if (parent === null) continue;
      for (const sib of parent.children) {
        if (sib !== el && (selector === undefined || matchesSelector(sib, selector))) out.push(sib);
      }
    }
    return new Q(unique(out));
  }

  /** Immediately following sibling, kept only when it matches `selector`. */
  next(selector?: string): Q<Element> {
    const out: Element[] = [];
    for (const el of this.els) {
      const n = el.nextElementSibling;
      if (n !== null && (selector === undefined || matchesSelector(n, selector))) out.push(n);
    }
    return new Q(out);
  }

  /** Immediately preceding sibling, kept only when it matches `selector`. */
  prev(selector?: string): Q<Element> {
    const out: Element[] = [];
    for (const el of this.els) {
      const p = el.previousElementSibling;
      if (p !== null && (selector === undefined || matchesSelector(p, selector))) out.push(p);
    }
    return new Q(out);
  }

  // -- manipulation ----------------------------------------------------

  /** Inner HTML of the first element (`undefined` when empty). */
  html(): string | undefined;
  /** Set inner HTML of every element (old data/listeners are purged). */
  html(value: string | ((this: T, index: number, old: string) => string)): this;
  html(value?: string | ((this: T, index: number, old: string) => string)): string | undefined | this {
    if (value === undefined) return this.els[0]?.innerHTML;
    let index = 0;
    for (const el of this.els) {
      purgeChildren(el);
      el.innerHTML = typeof value === 'function' ? value.call(el, index, el.innerHTML) : value;
      index++;
    }
    return this;
  }

  /** Combined text of all elements. */
  text(): string;
  /** Set text of every element (old data/listeners are purged). */
  text(value: string | ((this: T, index: number, old: string) => string)): this;
  text(value?: string | ((this: T, index: number, old: string) => string)): string | this {
    if (value === undefined) return this.els.map((el) => el.textContent ?? '').join('');
    let index = 0;
    for (const el of this.els) {
      purgeChildren(el);
      el.textContent = typeof value === 'function' ? value.call(el, index, el.textContent ?? '') : value;
      index++;
    }
    return this;
  }

  /** Insert content as the last child of every element. */
  append(content: Content): this {
    const nodes = normalizeContent(content);
    distribute([...this.els], nodes);
    return this;
  }

  /** Insert content as the first child of every element. */
  prepend(content: Content): this {
    const nodes = normalizeContent(content);
    const targets = [...this.els];
    targets.forEach((target, index) => {
      const last = index === targets.length - 1;
      const first = target.firstChild;
      for (const node of nodes) {
        target.insertBefore(last ? node : node.cloneNode(true), first);
      }
    });
    return this;
  }

  /** Insert content before every element. */
  before(content: Content): this {
    const nodes = normalizeContent(content);
    const targets = [...this.els];
    targets.forEach((target, index) => {
      const last = index === targets.length - 1;
      const parent = target.parentNode;
      if (parent === null) return;
      for (const node of nodes) {
        parent.insertBefore(last ? node : node.cloneNode(true), target);
      }
    });
    return this;
  }

  /** Insert content after every element. */
  after(content: Content): this {
    const nodes = normalizeContent(content);
    const targets = [...this.els];
    targets.forEach((target, index) => {
      const last = index === targets.length - 1;
      const parent = target.parentNode;
      if (parent === null) return;
      const next = target.nextSibling;
      for (const node of nodes) {
        parent.insertBefore(last ? node : node.cloneNode(true), next);
      }
    });
    return this;
  }

  /** Append these elements to each target. Returns the inserted set. */
  appendTo(target: string | Element | Q<Element>): Q<T> {
    return this.insertInto(target, 'append');
  }

  /** Prepend these elements to each target. Returns the inserted set. */
  prependTo(target: string | Element | Q<Element>): Q<T> {
    return this.insertInto(target, 'prepend');
  }

  /** Insert these elements before each target. Returns the inserted set. */
  insertBefore(target: string | Element | Q<Element>): Q<T> {
    return this.insertInto(target, 'before');
  }

  /** Insert these elements after each target. Returns the inserted set. */
  insertAfter(target: string | Element | Q<Element>): Q<T> {
    return this.insertInto(target, 'after');
  }

  private insertInto(
    target: string | Element | Q<Element>,
    mode: 'append' | 'prepend' | 'before' | 'after'
  ): Q<T> {
    const targets: Element[] =
      typeof target === 'string'
        ? [...document.querySelectorAll(target)]
        : target instanceof Q
          ? target.toArray()
          : [target];
    const inserted: T[] = [];
    targets.forEach((targetEl, index) => {
      const last = index === targets.length - 1;
      for (const el of this.els) {
        const node: Node = last ? el : el.cloneNode(true);
        if (!(node instanceof Element)) continue;
        if (node !== el) cloneData(el, node);
        if (mode === 'append') {
          targetEl.appendChild(node);
        } else if (mode === 'prepend') {
          targetEl.insertBefore(node, targetEl.firstChild);
        } else {
          const parent = targetEl.parentNode;
          if (parent === null) continue;
          parent.insertBefore(node, mode === 'before' ? targetEl : targetEl.nextSibling);
        }
        inserted.push(node as T);
      }
    });
    return new Q(inserted);
  }

  /**
   * Remove every element from the DOM, purging its data and listeners.
   * When `selector` is given, only matching elements are removed.
   */
  remove(selector?: string): this {
    for (const el of this.els) {
      if (selector !== undefined && !matchesSelector(el, selector)) continue;
      purgeData(el);
      purgeListenersDeep(el);
      el.remove();
    }
    return this;
  }

  /** Remove every element but keep its data and listeners. */
  detach(selector?: string): Q<T> {
    const out: T[] = [];
    for (const el of this.els) {
      if (selector !== undefined && !matchesSelector(el, selector)) continue;
      el.remove();
      out.push(el);
    }
    return new Q(out);
  }

  /** Remove all children (purging their data and listeners). */
  empty(): this {
    for (const el of this.els) {
      purgeChildren(el);
      el.replaceChildren();
    }
    return this;
  }

  /**
   * Deep-clone every element. Data is always copied; listeners are copied
   * too when `withListeners` is true (jQuery's `clone(dataAndEvents)`).
   */
  clone(withListeners = false): Q<T> {
    const out: T[] = [];
    for (const el of this.els) {
      const copy = el.cloneNode(true);
      if (!(copy instanceof Element)) continue;
      cloneDataDeep(el, copy);
      if (withListeners) cloneListenersDeep(el, copy);
      out.push(copy as T);
    }
    return new Q(out);
  }

  /** Replace every element with `content`. */
  replaceWith(content: Content): this {
    const nodes = normalizeContent(content);
    const targets = [...this.els];
    targets.forEach((target, index) => {
      const last = index === targets.length - 1;
      const parent = target.parentNode;
      if (parent === null) return;
      purgeData(target);
      purgeListenersDeep(target);
      for (const node of nodes) {
        parent.insertBefore(last ? node : node.cloneNode(true), target);
      }
      target.remove();
    });
    return this;
  }

  // -- attributes, properties, values, data -----------------------------

  /** Attribute value of the first element (`undefined` when missing). */
  attr(name: string): string | undefined;
  /** Set an attribute (`null` removes it). */
  attr(name: string, value: string | number | boolean | null): this;
  /** Set several attributes at once. */
  attr(values: Record<string, string | number | boolean | null>): this;
  attr(
    nameOrValues: string | Record<string, string | number | boolean | null>,
    value?: string | number | boolean | null
  ): string | undefined | this {
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

  /** Remove one or more (space-separated) attributes. */
  removeAttr(names: string): this {
    const list = names.split(/\s+/).filter((n) => n !== '');
    for (const el of this.els) {
      for (const name of list) el.removeAttribute(name);
    }
    return this;
  }

  /** Typed DOM property of the first element. */
  prop<K extends keyof T>(name: K): T[K] | undefined;
  /** Set a typed DOM property on every element. */
  prop<K extends keyof T>(name: K, value: T[K]): this;
  /** Set several properties at once. */
  prop(values: { [K in keyof T]?: T[K] }): this;
  prop<K extends keyof T>(
    nameOrValues: K | { [J in keyof T]?: T[J] },
    value?: T[K]
  ): T[K] | undefined | this {
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

  /**
   * Value of the first form element. Multi-selects yield an array,
   * anything else yields `undefined`.
   */
  val(): string | string[] | undefined;
  /** Set the value of input/select/textarea elements. */
  val(value: string | number | string[] | null): this;
  val(value?: string | number | string[] | null): string | string[] | undefined | this {
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

  /** Stored data (or coerced `data-*` attribute) for `key`. */
  data<U = unknown>(key: string): U | undefined;
  /** All data including `data-*` attributes (camelCase keys). */
  data(): Record<string, unknown>;
  /** Store one value. */
  data(key: string, value: unknown): this;
  /** Store several values. */
  data(values: Record<string, unknown>): this;
  data(
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

  /** Drop stored data (`key` omitted clears everything for these elements). */
  removeData(key?: string | string[]): this {
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
  }

  // -- classes, style, visibility ----------------------------------------

  private eachClassName(
    value: string | string[] | ((this: T, index: number, current: string) => string),
    apply: (el: T, names: string[]) => void
  ): this {
    let index = 0;
    for (const el of this.els) {
      const raw = typeof value === 'function' ? value.call(el, index, el.className) : value;
      const names = (Array.isArray(raw) ? raw.join(' ') : raw).split(/\s+/).filter((n) => n !== '');
      apply(el, names);
      index++;
    }
    return this;
  }

  /** Add classes (string, list, or per-element function). */
  addClass(value: string | string[] | ((this: T, index: number, current: string) => string)): this {
    return this.eachClassName(value, (el, names) => {
      if (names.length > 0) el.classList.add(...names);
    });
  }

  /** Remove classes (omitted clears all). */
  removeClass(
    value?: string | string[] | ((this: T, index: number, current: string) => string)
  ): this {
    if (value === undefined) {
      for (const el of this.els) el.className = '';
      return this;
    }
    return this.eachClassName(value, (el, names) => {
      if (names.length > 0) el.classList.remove(...names);
    });
  }

  /** Toggle classes (`force` pins the outcome). */
  toggleClass(
    value: string | string[] | ((this: T, index: number, current: string) => string),
    force?: boolean
  ): this {
    return this.eachClassName(value, (el, names) => {
      for (const name of names) el.classList.toggle(name, force);
    });
  }

  /** True when the first element carries `name`. */
  hasClass(name: string): boolean {
    const first = this.els[0];
    return first !== undefined && first.classList.contains(name);
  }

  /** Computed style value of the first element. */
  css(property: string): string | undefined;
  /** Set one style property (numbers gain `px` unless unitless). */
  css(property: string, value: string | number): this;
  /** Set several style properties at once. */
  css(values: Record<string, string | number>): this;
  css(
    propertyOrValues: string | Record<string, string | number>,
    value?: string | number
  ): string | undefined | this {
    if (typeof propertyOrValues === 'string' && value === undefined) {
      const first = this.els[0];
      if (first === undefined) return undefined;
      return getComputedStyle(first).getPropertyValue(cssPropertyName(propertyOrValues));
    }
    const entries: Array<[string, string | number]> =
      typeof propertyOrValues === 'string'
        ? [[propertyOrValues, value ?? '']]
        : Object.entries(propertyOrValues);
    for (const el of this.els) {
      const target = el instanceof HTMLElement || el instanceof SVGElement ? el : null;
      if (!target) continue;
      for (const [property, v] of entries) {
        target.style.setProperty(cssPropertyName(property), typeof v === 'number' ? addPx(property, v) : v);
      }
    }
    return this;
  }

  /** Show every element (restores the pre-hide display). */
  show(): this;
  /** Animated show (opacity). Resolves when done. */
  show(duration: number | EffectOptions): Promise<void>;
  show(duration?: number | EffectOptions): this | Promise<void> {
    if (duration === undefined) {
      for (const el of this.els) showInstant(el);
      return this;
    }
    const options = normalizeEffectOptions(duration);
    return Promise.all(this.els.map((el) => animateShowHide(el, true, options))).then(() => undefined);
  }

  /** Hide every element (caches display for show()). */
  hide(): this;
  /** Animated hide (opacity, then display:none). Resolves when done. */
  hide(duration: number | EffectOptions): Promise<void>;
  hide(duration?: number | EffectOptions): this | Promise<void> {
    if (duration === undefined) {
      for (const el of this.els) hideInstant(el);
      return this;
    }
    const options = normalizeEffectOptions(duration);
    return Promise.all(this.els.map((el) => animateShowHide(el, false, options))).then(() => undefined);
  }

  /** Toggle visibility instantly (`force` pins the outcome). */
  toggle(force?: boolean): this;
  /** Animated toggle. Resolves when done. */
  toggle(duration: number | EffectOptions): Promise<void>;
  toggle(forceOrDuration?: boolean | number | EffectOptions): this | Promise<void> {
    if (typeof forceOrDuration === 'number' || typeof forceOrDuration === 'object') {
      const options = normalizeEffectOptions(forceOrDuration);
      return Promise.all(
        this.els.map((el) => animateShowHide(el, isHidden(el), options))
      ).then(() => undefined);
    }
    for (const el of this.els) {
      const show = forceOrDuration ?? isHidden(el);
      if (show) showInstant(el);
      else hideInstant(el);
    }
    return this;
  }

  // -- events ------------------------------------------------------------

  /** Subscribe (typed event object). Space-separated types allowed. */
  on<K extends keyof HTMLElementEventMap>(
    type: K,
    handler: (this: T, event: HTMLElementEventMap[K]) => void
  ): this;
  /** Subscribe with delegation (`this` is the matched descendant). */
  on<K extends keyof HTMLElementEventMap>(
    type: K,
    selector: string,
    handler: (this: T, event: HTMLElementEventMap[K]) => void
  ): this;
  /** Subscribe to custom/unknown event types. */
  on(type: string, handler: (this: T, event: Event) => void): this;
  /** Custom types with delegation. */
  on(type: string, selector: string, handler: (this: T, event: Event) => void): this;
  on(
    type: string,
    selectorOrHandler: string | ((this: T, event: never) => void),
    handler?: (this: T, event: never) => void
  ): this {
    const selector = typeof selectorOrHandler === 'string' ? selectorOrHandler : undefined;
    const fn = (typeof selectorOrHandler === 'string' ? handler : selectorOrHandler) as
      | EventHandler
      | undefined;
    if (fn === undefined) return this;
    for (const el of this.els) {
      addListener(el, type, selector, fn);
    }
    return this;
  }

  /** Subscribe once (typed event object). */
  one<K extends keyof HTMLElementEventMap>(
    type: K,
    handler: (this: T, event: HTMLElementEventMap[K]) => void
  ): this;
  /** Subscribe once with delegation. */
  one<K extends keyof HTMLElementEventMap>(
    type: K,
    selector: string,
    handler: (this: T, event: HTMLElementEventMap[K]) => void
  ): this;
  /** Subscribe once to custom/unknown event types. */
  one(type: string, handler: (this: T, event: Event) => void): this;
  /** Subscribe once to custom types with delegation. */
  one(type: string, selector: string, handler: (this: T, event: Event) => void): this;
  one(
    type: string,
    selectorOrHandler: string | ((this: T, event: never) => void),
    handler?: (this: T, event: never) => void
  ): this {
    const selector = typeof selectorOrHandler === 'string' ? selectorOrHandler : undefined;
    const fn = (typeof selectorOrHandler === 'string' ? handler : selectorOrHandler) as
      | EventHandler
      | undefined;
    if (fn === undefined) return this;
    for (const el of this.els) {
      addListener(el, type, selector, fn, { once: true });
    }
    return this;
  }

  /** Unsubscribe. Omitted criteria are wildcards (`off()` clears all). */
  off(): this;
  off(type: string): this;
  off(type: string, selector: string): this;
  off(type: string, handler: (...args: never[]) => unknown): this;
  off(type: string, selector: string, handler: (...args: never[]) => unknown): this;
  off(type?: string, selectorOrHandler?: string | ((...args: never[]) => unknown), handler?: (...args: never[]) => unknown): this {
    const selector = typeof selectorOrHandler === 'string' ? selectorOrHandler : undefined;
    const fn = (
      typeof selectorOrHandler === 'string' ? handler : selectorOrHandler
    ) as EventHandler | undefined;
    for (const el of this.els) {
      removeListener(el, type, selector, fn);
    }
    return this;
  }

  /**
   * Dispatch a bubbling event. DEVIATION: always a CustomEvent carrying
   * `detail` (jQuery synthesizes per-type event objects); namespaces in
   * `type` are ignored for filtering.
   */
  trigger<K extends keyof HTMLElementEventMap>(type: K, detail?: unknown): this;
  trigger(type: string, detail?: unknown): this;
  trigger(type: string, detail?: unknown): this {
    const base = type.split('.')[0] ?? type;
    for (const el of this.els) {
      el.dispatchEvent(new CustomEvent(base, { bubbles: true, cancelable: true, detail }));
    }
    return this;
  }

  /** Shorthand for mouseenter/mouseleave (single handler covers both). */
  hover(
    over: (this: T, event: MouseEvent) => void,
    out?: (this: T, event: MouseEvent) => void
  ): this {
    const leave = out ?? over;
    return this.on('mouseenter', over).on('mouseleave', leave);
  }

  // -- effects (Promise-based; see effects.ts) ----------------------------

  /** Fade every element in. Resolves when done. */
  fadeIn(duration?: number | EffectOptions): Promise<void> {
    const options = normalizeEffectOptions(duration);
    return Promise.all(
      this.els.map((el) => {
        showInstant(el);
        return animateOpacity(el, 0, 1, options);
      })
    ).then(() => undefined);
  }

  /** Fade every element out (then display:none). Resolves when done. */
  fadeOut(duration?: number | EffectOptions): Promise<void> {
    const options = normalizeEffectOptions(duration);
    return Promise.all(
      this.els.map((el) =>
        animateOpacity(el, null, 0, options).then(() => {
          hideInstant(el);
        })
      )
    ).then(() => undefined);
  }

  /** Fade to `opacity` (0–1). Resolves when done. */
  fadeTo(opacity: number, duration?: number | EffectOptions): Promise<void> {
    const options = normalizeEffectOptions(duration);
    return Promise.all(this.els.map((el) => animateOpacity(el, null, opacity, options))).then(
      () => undefined
    );
  }

  /** Fade in/out depending on current visibility. Resolves when done. */
  fadeToggle(duration?: number | EffectOptions): Promise<void> {
    const options = normalizeEffectOptions(duration);
    return Promise.all(
      this.els.map((el) => {
        if (isHidden(el)) {
          showInstant(el);
          return animateOpacity(el, 0, 1, options);
        }
        return animateOpacity(el, null, 0, options).then(() => {
          hideInstant(el);
        });
      })
    ).then(() => undefined);
  }

  /** Slide every element down. Resolves when done. */
  slideDown(duration?: number | EffectOptions): Promise<void> {
    const options = normalizeEffectOptions(duration);
    return Promise.all(this.els.map((el) => animateSlide(el, true, options))).then(() => undefined);
  }

  /** Slide every element up (then display:none). Resolves when done. */
  slideUp(duration?: number | EffectOptions): Promise<void> {
    const options = normalizeEffectOptions(duration);
    return Promise.all(this.els.map((el) => animateSlide(el, false, options))).then(() => undefined);
  }

  /** Slide down/up depending on current visibility. Resolves when done. */
  slideToggle(duration?: number | EffectOptions): Promise<void> {
    const options = normalizeEffectOptions(duration);
    return Promise.all(this.els.map((el) => animateSlide(el, isHidden(el), options))).then(
      () => undefined
    );
  }

  /** Run raw WAAPI keyframes on every element. Resolves when done. */
  animate(
    keyframes: Keyframe[] | PropertyIndexedKeyframes,
    duration?: number | EffectOptions
  ): Promise<void> {
    const options = normalizeEffectOptions(duration);
    return Promise.all(this.els.map((el) => runAnimation(el, keyframes, options))).then(
      () => undefined
    );
  }

  /** Cancel running animations (`gotoEnd` jumps to end state). */
  stop(gotoEnd = false): this {
    for (const el of this.els) stopAnimations(el, gotoEnd);
    return this;
  }
}

// -- css helpers ---------------------------------------------------------

/** jQuery.cssNumber: properties that stay unitless when set with a number. */
const UNITLESS = new Set([
  'animationIterationCount',
  'aspectRatio',
  'borderImageOutset',
  'borderImageSlice',
  'borderImageWidth',
  'boxFlex',
  'boxFlexGroup',
  'boxOrdinalGroup',
  'columnCount',
  'columns',
  'flex',
  'flexGrow',
  'flexPositive',
  'flexShrink',
  'flexNegative',
  'flexOrder',
  'fontWeight',
  'gridArea',
  'gridColumn',
  'gridColumnEnd',
  'gridColumnStart',
  'gridRow',
  'gridRowEnd',
  'gridRowStart',
  'lineClamp',
  'lineHeight',
  'opacity',
  'order',
  'orphans',
  'scale',
  'widows',
  'zIndex',
  'zoom',
  'fillOpacity',
  'floodOpacity',
  'stopOpacity',
  'strokeMiterlimit',
  'strokeOpacity',
  'strokeWidth',
]);

/** setProperty/getPropertyValue need kebab-case (custom props as-is). */
function cssPropertyName(property: string): string {
  if (property.startsWith('--')) return property;
  return property.replace(/[A-Z]/g, (ch) => `-${ch.toLowerCase()}`);
}

function addPx(property: string, value: number): string {
  if (UNITLESS.has(property)) return String(value);
  return `${value}px`;
}

function normalizeEffectOptions(duration?: number | EffectOptions): EffectOptions {
  if (duration === undefined) return {};
  return typeof duration === 'number' ? { duration } : duration;
}

// -- factory --------------------------------------------------------------

/** Roots a selector query can run against. */
export type QueryContext = ParentNode | Q<Element> | null | undefined;

function resolveRoots(context: QueryContext): ParentNode[] {
  if (context === null || context === undefined) return [document];
  if (context instanceof Q) {
    const roots = context.toArray();
    return roots.length > 0 ? roots : [document];
  }
  return [context];
}

function isCreationString(selector: string): boolean {
  return selector.trimStart().startsWith('<');
}

/**
 * Select elements. A tag-name literal (`q('input')`) yields a precisely
 * typed set; anything else yields `Q<Element>` (or name the type:
 * `q<HTMLDivElement>('.card')`). Strings starting with `<` create elements.
 */
export function q<K extends keyof HTMLElementTagNameMap>(
  selector: K,
  context?: QueryContext
): Q<HTMLElementTagNameMap[K]>;
export function q<T extends Element = Element>(selector: string, context?: QueryContext): Q<T>;
export function q<T extends Element>(element: T | Q<T> | null | undefined): Q<T>;
export function q<T extends Element>(elements: ArrayLike<T>): Q<T>;
export function q(
  selectorOrElements:
    | string
    | Element
    | Q<Element>
    | ArrayLike<Element>
    | null
    | undefined,
  context?: QueryContext
): Q<Element> {
  if (selectorOrElements === null || selectorOrElements === undefined) return new Q([]);
  if (selectorOrElements instanceof Q) return new Q(selectorOrElements.toArray());
  if (selectorOrElements instanceof Element) return new Q([selectorOrElements]);
  if (typeof selectorOrElements !== 'string') return new Q(Array.from(selectorOrElements));
  if (isCreationString(selectorOrElements)) {
    const template = document.createElement('template');
    template.innerHTML = selectorOrElements.trim();
    return new Q([...template.content.children]);
  }
  const out: Element[] = [];
  for (const root of resolveRoots(context)) {
    out.push(...root.querySelectorAll(selectorOrElements));
  }
  return new Q(unique(out));
}

/**
 * Look up one element by id (exact `getElementById` semantics, wrapped
 * as a set). Unlike `q('#' + id)` this is safe for ids that are not
 * valid CSS selectors (leading digits, user-controlled folder names).
 */
export function byId<T extends Element = Element>(id: string): Q<T> {
  const el = document.getElementById(id);
  return new Q(el === null ? [] : [el as unknown as T]);
}

/** Run `fn` once the DOM is ready (async even when already loaded). */
export function ready(fn: () => void): void {
  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', fn, { once: true });
  } else {
    queueMicrotask(fn);
  }
}
