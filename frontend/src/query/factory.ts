// Q factory: q/byId (extracted from core.ts).

import { Q, unique } from './core';

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
