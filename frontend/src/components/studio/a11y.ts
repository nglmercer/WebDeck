/** Each ModalShell owns and disconnects its focus observer and keyboard trap. */

const CONTAINER_SELECTOR =
  '.modal-container, .addbutton-modal-container, ' +
  '.addbutton-modal-container-args, .editbutton-modal-container';

function isShown(container: HTMLElement): boolean {
  return container.style.display === 'block';
}

/** Wire focus open/restore for every modal under `root` (once per container). */
export function wireModalA11y(root: ParentNode = document): () => void {
  const cleanups: Array<() => void> = [];
  const containers = root instanceof HTMLElement && root.matches(CONTAINER_SELECTOR) ? [root] : [...root.querySelectorAll(CONTAINER_SELECTOR)] as HTMLElement[];
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
    cleanups.push(() => { observer.disconnect(); delete container.dataset.wd2A11y; });
    const trap = (event: KeyboardEvent): void => {
      if (event.key !== 'Tab' || !isShown(container) || !container.contains(document.activeElement)) return;
      const focusable = [...dialog.querySelectorAll<HTMLElement>('button, input, select, textarea, a[href], [tabindex="0"]')].filter(el => !el.hasAttribute('disabled') && el.getClientRects().length > 0);
      const first = focusable[0]; const last = focusable[focusable.length - 1];
      if (!first) { event.preventDefault(); dialog.focus(); }
      else if (event.shiftKey && (document.activeElement === first || document.activeElement === dialog)) { event.preventDefault(); last?.focus(); }
      else if (!event.shiftKey && (document.activeElement === last || document.activeElement === dialog)) { event.preventDefault(); first.focus(); }
    };
    container.addEventListener('keydown', trap);
    cleanups.push(() => container.removeEventListener('keydown', trap));
    observer.observe(container, { attributes: true, attributeFilter: ['style'] });
  }
  return () => cleanups.forEach(cleanup => cleanup());
}
