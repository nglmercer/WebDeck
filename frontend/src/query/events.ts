// Event registry: typed on() with delegation matching, plus listener
// purge on remove(). Entries are keyed by element in a WeakMap, so
// dropped elements never leak registrations.

import { Q } from './core';

/** Loosely-typed stored handler (public overloads narrow per event). */
type EventHandler = (this: Element, event: Event) => void;

interface Registration {
  base: string;
  namespaces: string[];
  selector: string | undefined;
  handler: EventHandler;
  wrapped: EventListener;
  capture: boolean;
  once: boolean;
}

const registry = new WeakMap<Element, Registration[]>();

function parseType(type: string): { base: string; namespaces: string[] } {
  const [base = '', ...namespaces] = type.split('.');
  return { base, namespaces };
}

function registrationsFor(el: Element): Registration[] {
  let list = registry.get(el);
  if (!list) {
    list = [];
    registry.set(el, list);
  }
  return list;
}

interface ListenerOptions {
  once?: boolean;
  capture?: boolean;
}

/** Register `handler` for each space-separated type (namespace-aware). */
function addListener(
  el: Element,
  type: string,
  selector: string | undefined,
  handler: EventHandler,
  options?: ListenerOptions
): void {
  const capture = options?.capture ?? false;
  const once = options?.once ?? false;
  for (const single of type.split(/\s+/).filter((t) => t !== '')) {
    const { base, namespaces } = parseType(single);
    if (base === '') continue;
    const wrapped: EventListener = (event) => {
      if (selector === undefined) {
        handler.call(el, event);
        return;
      }
      const target = event.target;
      if (!(target instanceof Element)) return;
      const match = target.closest(selector);
      if (match !== null && el.contains(match)) handler.call(match, event);
    };
    el.addEventListener(base, wrapped, { capture, once });
    registrationsFor(el).push({ base, namespaces, selector, handler, wrapped, capture, once });
  }
}

/** Detach every registered listener in a subtree (jQuery.cleanData). */
export function purgeListenersDeep(root: Element): void {
  purgeOne(root);
  for (const child of root.querySelectorAll('*')) {
    purgeOne(child);
  }
}

function purgeOne(el: Element): void {
  const list = registry.get(el);
  if (!list) return;
  for (const reg of list) {
    el.removeEventListener(reg.base, reg.wrapped, { capture: reg.capture });
  }
  registry.delete(el);
}

// -- Q event methods (extracted from core.ts) --------------------------------

declare module './core' {
  interface Q<T extends Element> {
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
  }
}

function onImpl<T extends Element>(
  this: Q<T>,
  type: string,
  selectorOrHandler: string | ((this: T, event: never) => void),
  handler?: (this: T, event: never) => void
): Q<T> {
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

Q.prototype.on = onImpl as Q<Element>['on'];
