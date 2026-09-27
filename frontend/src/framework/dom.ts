// Typed DOM query helpers (the ported wire-up code is query-heavy).

export function qs<T extends Element = Element>(root: ParentNode, selector: string): T | null {
  return root.querySelector<T>(selector);
}

export function qsa<T extends Element = Element>(root: ParentNode, selector: string): T[] {
  return Array.from(root.querySelectorAll<T>(selector));
}

export function requireEl<T extends Element = Element>(root: ParentNode, selector: string): T {
  const el = qs<T>(root, selector);
  if (!el) throw new Error(`missing element: ${selector}`);
  return el;
}

export function on<K extends keyof HTMLElementEventMap>(
  el: Element,
  event: K,
  handler: (ev: HTMLElementEventMap[K]) => void,
  options?: AddEventListenerOptions
): void {
  el.addEventListener(event, handler as EventListener, options);
}
