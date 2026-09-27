// Q DOM-manipulation methods (extracted from core.ts).

import { Q, matchesSelector } from './core';
import type { Content } from './core';
import { cloneData, cloneDataDeep, purgeChildren, purgeData } from './data';
import { cloneListenersDeep, purgeListenersDeep } from './events';

declare module './core' {
  interface Q<T extends Element> {
    /** Inner HTML of the first element (`undefined` when empty). */
    html(): string | undefined;
    /** Set inner HTML of every element (old data/listeners are purged). */
    html(value: string | ((this: T, index: number, old: string) => string)): this;
    /** Combined text of all elements. */
    text(): string;
    /** Set text of every element (old data/listeners are purged). */
    text(value: string | ((this: T, index: number, old: string) => string)): this;
    /** Insert content as the last child of every element. */
    append(content: Content): this;
    /** Insert content as the first child of every element. */
    prepend(content: Content): this;
    /** Insert content before every element. */
    before(content: Content): this;
    /** Insert content after every element. */
    after(content: Content): this;
    /** Append these elements to each target. Returns the inserted set. */
    appendTo(target: string | Element | Q<Element>): Q<T>;
    /** Prepend these elements to each target. Returns the inserted set. */
    prependTo(target: string | Element | Q<Element>): Q<T>;
    /** Insert these elements before each target. Returns the inserted set. */
    insertBefore(target: string | Element | Q<Element>): Q<T>;
    /** Insert these elements after each target. Returns the inserted set. */
    insertAfter(target: string | Element | Q<Element>): Q<T>;
    /**
     * Remove every element from the DOM, purging its data and listeners.
     * When `selector` is given, only matching elements are removed.
     */
    remove(selector?: string): this;
    /** Remove every element but keep its data and listeners. */
    detach(selector?: string): Q<T>;
    /** Remove all children (purging their data and listeners). */
    empty(): this;
    /**
     * Deep-clone every element. Data is always copied; listeners are copied
     * too when `withListeners` is true (jQuery's `clone(dataAndEvents)`).
     */
    clone(withListeners?: boolean): Q<T>;
    /** Replace every element with `content`. */
    replaceWith(content: Content): this;
  }
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

function insertInto<T extends Element>(
  els: readonly T[],
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
    for (const el of els) {
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

function htmlImpl<T extends Element>(
  this: Q<T>,
  value?: string | ((this: T, index: number, old: string) => string)
): string | undefined | Q<T> {
  if (value === undefined) return this.els[0]?.innerHTML;
  let index = 0;
  for (const el of this.els) {
    purgeChildren(el);
    el.innerHTML = typeof value === 'function' ? value.call(el, index, el.innerHTML) : value;
    index++;
  }
  return this;
}

Q.prototype.html = htmlImpl as Q<Element>['html'];

function textImpl<T extends Element>(
  this: Q<T>,
  value?: string | ((this: T, index: number, old: string) => string)
): string | Q<T> {
  if (value === undefined) return this.els.map((el) => el.textContent ?? '').join('');
  let index = 0;
  for (const el of this.els) {
    purgeChildren(el);
    el.textContent = typeof value === 'function' ? value.call(el, index, el.textContent ?? '') : value;
    index++;
  }
  return this;
}

Q.prototype.text = textImpl as Q<Element>['text'];

Q.prototype.append = function <T extends Element>(this: Q<T>, content: Content): Q<T> {
  const nodes = normalizeContent(content);
  distribute([...this.els], nodes);
  return this;
};

Q.prototype.prepend = function <T extends Element>(this: Q<T>, content: Content): Q<T> {
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
};

Q.prototype.before = function <T extends Element>(this: Q<T>, content: Content): Q<T> {
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
};

Q.prototype.after = function <T extends Element>(this: Q<T>, content: Content): Q<T> {
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
};

Q.prototype.appendTo = function <T extends Element>(
  this: Q<T>,
  target: string | Element | Q<Element>
): Q<T> {
  return insertInto(this.els, target, 'append');
};

Q.prototype.prependTo = function <T extends Element>(
  this: Q<T>,
  target: string | Element | Q<Element>
): Q<T> {
  return insertInto(this.els, target, 'prepend');
};

Q.prototype.insertBefore = function <T extends Element>(
  this: Q<T>,
  target: string | Element | Q<Element>
): Q<T> {
  return insertInto(this.els, target, 'before');
};

Q.prototype.insertAfter = function <T extends Element>(
  this: Q<T>,
  target: string | Element | Q<Element>
): Q<T> {
  return insertInto(this.els, target, 'after');
};

Q.prototype.remove = function <T extends Element>(this: Q<T>, selector?: string): Q<T> {
  for (const el of this.els) {
    if (selector !== undefined && !matchesSelector(el, selector)) continue;
    purgeData(el);
    purgeListenersDeep(el);
    el.remove();
  }
  return this;
};

Q.prototype.detach = function <T extends Element>(this: Q<T>, selector?: string): Q<T> {
  const out: T[] = [];
  for (const el of this.els) {
    if (selector !== undefined && !matchesSelector(el, selector)) continue;
    el.remove();
    out.push(el);
  }
  return new Q(out);
};

Q.prototype.empty = function <T extends Element>(this: Q<T>): Q<T> {
  for (const el of this.els) {
    purgeChildren(el);
    el.replaceChildren();
  }
  return this;
};

Q.prototype.clone = function <T extends Element>(this: Q<T>, withListeners = false): Q<T> {
  const out: T[] = [];
  for (const el of this.els) {
    const copy = el.cloneNode(true);
    if (!(copy instanceof Element)) continue;
    cloneDataDeep(el, copy);
    if (withListeners) cloneListenersDeep(el, copy);
    out.push(copy as T);
  }
  return new Q(out);
};

Q.prototype.replaceWith = function <T extends Element>(this: Q<T>, content: Content): Q<T> {
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
};
