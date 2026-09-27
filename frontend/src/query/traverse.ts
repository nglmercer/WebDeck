// Q tree-walking methods (extracted from core.ts).

import { Q, matchesSelector, unique } from './core';

declare module './core' {
  interface Q<T extends Element> {
    /** Descendants matching `selector` (deduped, document order). */
    find(selector: string): Q<Element>;
    /** Direct child elements, optionally filtered. */
    children(selector?: string): Q<Element>;
    /** Unique immediate parents, optionally filtered. */
    parent(selector?: string): Q<Element>;
    /** All ancestors (nearest first), optionally filtered. */
    parents(selector?: string): Q<Element>;
    /** First ancestor-or-self matching `selector`. */
    closest(selector: string): Q<Element>;
    /** Sibling elements (excluding self), optionally filtered. */
    siblings(selector?: string): Q<Element>;
    /** Immediately following sibling, kept only when it matches `selector`. */
    next(selector?: string): Q<Element>;
    /** Immediately preceding sibling, kept only when it matches `selector`. */
    prev(selector?: string): Q<Element>;
  }
}

Q.prototype.find = function (this: Q, selector: string): Q<Element> {
  const out: Element[] = [];
  for (const el of this.els) {
    out.push(...el.querySelectorAll(selector));
  }
  return new Q(unique(out));
};

Q.prototype.children = function (this: Q, selector?: string): Q<Element> {
  const out: Element[] = [];
  for (const el of this.els) {
    for (const child of el.children) {
      if (selector === undefined || matchesSelector(child, selector)) out.push(child);
    }
  }
  return new Q(out);
};

Q.prototype.parent = function (this: Q, selector?: string): Q<Element> {
  const out: Element[] = [];
  for (const el of this.els) {
    const p = el.parentElement;
    if (p !== null && (selector === undefined || matchesSelector(p, selector))) out.push(p);
  }
  return new Q(unique(out));
};

Q.prototype.parents = function (this: Q, selector?: string): Q<Element> {
  const out: Element[] = [];
  for (const el of this.els) {
    let p = el.parentElement;
    while (p !== null) {
      if (selector === undefined || matchesSelector(p, selector)) out.push(p);
      p = p.parentElement;
    }
  }
  return new Q(unique(out));
};

Q.prototype.closest = function (this: Q, selector: string): Q<Element> {
  const out: Element[] = [];
  for (const el of this.els) {
    const found = el.closest(selector);
    if (found !== null) out.push(found);
  }
  return new Q(unique(out));
};

Q.prototype.siblings = function (this: Q, selector?: string): Q<Element> {
  const out: Element[] = [];
  for (const el of this.els) {
    const parent = el.parentElement;
    if (parent === null) continue;
    for (const sib of parent.children) {
      if (sib !== el && (selector === undefined || matchesSelector(sib, selector))) out.push(sib);
    }
  }
  return new Q(unique(out));
};

Q.prototype.next = function (this: Q, selector?: string): Q<Element> {
  const out: Element[] = [];
  for (const el of this.els) {
    const n = el.nextElementSibling;
    if (n !== null && (selector === undefined || matchesSelector(n, selector))) out.push(n);
  }
  return new Q(out);
};

Q.prototype.prev = function (this: Q, selector?: string): Q<Element> {
  const out: Element[] = [];
  for (const el of this.els) {
    const p = el.previousElementSibling;
    if (p !== null && (selector === undefined || matchesSelector(p, selector))) out.push(p);
  }
  return new Q(out);
};
