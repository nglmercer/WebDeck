// Q DOM-manipulation methods (extracted from core.ts).

import { Q, matchesSelector } from './core';
import type { Content } from './core';
import { purgeListenersDeep } from './events';

declare module './core' {
  interface Q<T extends Element> {
    /** Inner HTML of the first element (`undefined` when empty). */
    html(): string | undefined;
    /** Set inner HTML of every element (old listeners are purged). */
    html(value: string | ((this: T, index: number, old: string) => string)): this;
    /** Combined text of all elements. */
    text(): string;
    /** Set text of every element (old listeners are purged). */
    text(value: string | ((this: T, index: number, old: string) => string)): this;
    /** Insert content as the last child of every element. */
    append(content: Content): this;
    /**
     * Remove every element from the DOM, purging its listeners.
     * When `selector` is given, only matching elements are removed.
     */
    remove(selector?: string): this;
    /** Replace every element with `content`. */
    replaceWith(content: Content): this;
  }
}

/** Detach every registered listener in an element's children. */
function purgeChildren(el: Element): void {
  for (const child of [...el.children]) {
    purgeListenersDeep(child);
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

Q.prototype.remove = function <T extends Element>(this: Q<T>, selector?: string): Q<T> {
  for (const el of this.els) {
    if (selector !== undefined && !matchesSelector(el, selector)) continue;
    purgeListenersDeep(el);
    el.remove();
  }
  return this;
};

Q.prototype.replaceWith = function <T extends Element>(this: Q<T>, content: Content): Q<T> {
  const nodes = normalizeContent(content);
  const targets = [...this.els];
  targets.forEach((target, index) => {
    const last = index === targets.length - 1;
    const parent = target.parentNode;
    if (parent === null) return;
    purgeListenersDeep(target);
    for (const node of nodes) {
      parent.insertBefore(last ? node : node.cloneNode(true), target);
    }
    target.remove();
  });
  return this;
};
