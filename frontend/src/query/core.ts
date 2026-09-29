// qdom — a modern, typesafe jQuery replacement (zero dependencies).
//
// Why it is better than jQuery:
// - Generic element typing: `q('input')` is `Q<HTMLInputElement>`, so
//   `.val()`, `.prop('checked')`, … are checked per element type.
// - Typed events: `on('click', …)` narrows the event object via
//   `HTMLElementEventMap`; delegation keeps `this` typed too.
// - No silent footguns: index access is `T | undefined`, getters on empty
//   sets return `undefined`, and invalid selectors throw real errors.
// - Inserted HTML never executes scripts.
//
// The Q method groups live in sibling modules
// (traverse/manipulate/attributes/classes-css/events) and attach via
// declaration merging; `index.ts` imports them for side effects, and
// `factory.ts` owns `q`/`byId`.

/** Content accepted by insertion methods. */
export type Content = string | Element | Q<Element> | ArrayLike<string | Element>;

/** Predicate form for is(). */
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
}
