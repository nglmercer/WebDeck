/**
 * Set raw content attributes on an element (Svelte action).
 *
 * Escape hatch for upstream string event handlers (`onclick="folder(...)"`,
 * `onclickhandler="..."`): Svelte expresses handlers as functions (delegated,
 * running after directly-attached listeners), but the grid's inline handlers
 * must keep content-attribute semantics — they run at the target during
 * bubbling, before the editor's form-level `swapButton` listener, and
 * `swap.ts` reads the `onclickhandler` attribute back. `setAttribute` keeps
 * all of that byte-identical to parsed HTML. Render-once (no `update`).
 */
export function stringAttrs(node: HTMLElement, attrs: Record<string, string>): void {
  for (const [name, value] of Object.entries(attrs)) {
    node.setAttribute(name, value);
  }
}
