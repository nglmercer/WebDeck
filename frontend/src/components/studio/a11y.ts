/**
 * Dialog focus management for the studio modals.
 *
 * All modals mount hidden at boot and open imperatively (query `show_*`
 * helpers set inline `display: block`). A `MutationObserver` per modal
 * container watches that transition: on open, focus moves into the
 * dialog content (`tabindex="-1"`, never a tab stop); on close, focus
 * returns to the element that had it. No focus trap: stacked modals
 * (browser + args) stay navigable, and Escape already closes globally.
 */

const CONTAINER_SELECTOR =
  '.modal-container, .addbutton-modal-container, ' +
  '.addbutton-modal-container-args, .editbutton-modal-container';

function isShown(container: HTMLElement): boolean {
  return container.style.display === 'block';
}

/** Wire focus open/restore for every modal under `root` (once per container). */
export function wireModalA11y(root: ParentNode = document): void {
  const containers = [...root.querySelectorAll(CONTAINER_SELECTOR)] as HTMLElement[];
  for (const container of containers) {
    if (container.dataset.wd2A11y === '1') continue;
    container.dataset.wd2A11y = '1';
    const dialog = container.querySelector('[data-wd2-dialog]') as HTMLElement | null;
    if (!dialog) continue;
    let open = isShown(container);
    let lastFocus: Element | null = null;
    const observer = new MutationObserver(() => {
      const shown = isShown(container);
      if (shown === open) return;
      open = shown;
      if (shown) {
        lastFocus = document.activeElement;
        dialog.focus({ preventScroll: true });
      } else if (lastFocus instanceof HTMLElement && lastFocus.isConnected) {
        lastFocus.focus({ preventScroll: true });
        lastFocus = null;
      }
    });
    observer.observe(container, { attributes: true, attributeFilter: ['style'] });
  }
}
