// Event registry: enables off(), namespaces, delegation matching,
// listener purge on remove(), and listener cloning. Entries are keyed by
// element in a WeakMap, so dropped elements never leak registrations.

/** Loosely-typed stored handler (public overloads narrow per event). */
export type EventHandler = (this: Element, event: Event) => void;

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

export interface ListenerOptions {
  once?: boolean;
  capture?: boolean;
}

/** Register `handler` for each space-separated type (namespace-aware). */
export function addListener(
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

function matchesRegistration(
  reg: Registration,
  base: string,
  namespaces: string[],
  selector: string | undefined,
  selectorGiven: boolean,
  handler: EventHandler | undefined
): boolean {
  if (base !== '' && reg.base !== base) return false;
  for (const ns of namespaces) {
    if (!reg.namespaces.includes(ns)) return false;
  }
  if (selectorGiven && reg.selector !== selector) return false;
  if (handler !== undefined && reg.handler !== handler) return false;
  return true;
}

/**
 * Remove listeners. Omitted criteria are wildcards: `off()` clears
 * everything, `off('.ns')` clears a namespace across types, `off('click')`
 * clears direct and delegated click handlers alike.
 */
export function removeListener(
  el: Element,
  type?: string,
  selector?: string,
  handler?: EventHandler
): void {
  const list = registry.get(el);
  if (!list) return;
  const selectorGiven = selector !== undefined;
  const keep: Registration[] = [];
  for (const reg of list) {
    const drop =
      type === undefined
        ? selectorGiven || handler !== undefined
          ? matchesRegistration(reg, '', [], selector, selectorGiven, handler)
          : true
        : type
            .split(/\s+/)
            .filter((t) => t !== '')
            .some((single) => {
              const { base, namespaces } = parseType(single);
              return matchesRegistration(reg, base, namespaces, selector, selectorGiven, handler);
            });
    if (drop) {
      el.removeEventListener(reg.base, reg.wrapped, { capture: reg.capture });
    } else {
      keep.push(reg);
    }
  }
  if (keep.length > 0) registry.set(el, keep);
  else registry.delete(el);
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

/** Re-register one's listeners pairwise across two parallel subtrees. */
export function cloneListenersDeep(src: Element, dst: Element): void {
  cloneOne(src, dst);
  const srcKids = [...src.querySelectorAll('*')];
  const dstKids = [...dst.querySelectorAll('*')];
  srcKids.forEach((kid, index) => {
    const twin = dstKids[index];
    if (twin !== undefined) cloneOne(kid, twin);
  });
}

function cloneOne(src: Element, dst: Element): void {
  const list = registry.get(src);
  if (!list) return;
  for (const reg of list) {
    const type = reg.namespaces.length > 0 ? `${reg.base}.${reg.namespaces.join('.')}` : reg.base;
    addListener(dst, type, reg.selector, reg.handler, { once: reg.once, capture: reg.capture });
  }
}
