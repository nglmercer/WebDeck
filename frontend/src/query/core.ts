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
// Deliberate deviations from jQuery are marked "DEVIATION" in the method
// modules. The Q method groups live in sibling modules
// (traverse/manipulate/attributes/data/classes-css/events/effects) and
// attach via declaration merging; `index.ts` imports them for side
// effects, and `factory.ts` owns `q`/`byId`/`ready`.

/** Content accepted by insertion methods. */
export type Content = string | Element | Q<Element> | ArrayLike<string | Element>;

/** Predicate form shared by filter/not/is. */
export type ElementPredicate<T extends Element> = (this: T, index: number, element: T) => unknown;

/** Single matching primitive shared by filter/traverse/manipulate. */
export function matchesSelector(element: Element, selector: string): boolean {
  return element.matches(selector);
}

/** Dedup preserving first-seen order (document order for queries). */
export function unique<T extends Element>(elements: T[]): T[] {
  return [...new Set(elements)];
}

/**
 * Chainable, array-like wrapper around a fixed element list.
 * Empty sets are safe: setters/chainers no-op, getters return `undefined`.
 */
export class Q<T extends Element = Element> {
  /** @internal Fixed element list (method modules read this directly). */
  readonly els: readonly T[];

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
}
