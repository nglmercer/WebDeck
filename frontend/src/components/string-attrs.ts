/** Set dynamic data attributes for compatibility collectors; never executable handlers. */
export function stringAttrs(node: HTMLElement, attrs: Record<string, string>): void {
  for (const [name, value] of Object.entries(attrs)) {
    node.setAttribute(name, value);
  }
}
