import { tutorialActive } from '../features/learn/tour-state';

/** Native modal semantics plus deterministic Tab cycling and focus restoration. */
export function modal(dialog: HTMLDialogElement, options: { modal?: boolean } = {}) {
  const previous = document.activeElement instanceof HTMLElement ? document.activeElement : null;
  let touring = false;
  let modalMode = false;
  function display() {
    const next = options.modal !== false && !touring;
    dialog.toggleAttribute('data-tutorial-dialog', touring);
    if (dialog.open && next === modalMode) return;
    const focused = dialog.contains(document.activeElement)
      ? (document.activeElement as HTMLElement)
      : null;
    if (dialog.open) dialog.close();
    modalMode = next;
    if (next) dialog.showModal();
    else dialog.show();
    focused?.focus({ preventScroll: true });
  }
  const unsubscribe = tutorialActive.subscribe((active) => {
    touring = active;
    display();
  });
  const keydown = (event: KeyboardEvent) => {
    if (touring || options.modal === false || event.key !== 'Tab') return;
    const targets = [
      ...dialog.querySelectorAll<HTMLElement>(
        'button:not(:disabled), input:not(:disabled), select:not(:disabled), textarea:not(:disabled), a[href], summary, [tabindex]',
      ),
    ].filter(
      (element) =>
        element.tabIndex >= 0 &&
        element.getClientRects().length > 0 &&
        ![...dialog.querySelectorAll('details:not([open])')].some(
          (details) =>
            details.contains(element) && details.querySelector(':scope > summary') !== element,
        ),
    );
    const first = targets[0],
      last = targets.at(-1);
    if (!first || !last) {
      event.preventDefault();
      dialog.focus();
      return;
    }
    event.preventDefault();
    const index = targets.findIndex((target) => target === document.activeElement);
    const next =
      index < 0
        ? event.shiftKey
          ? targets.length - 1
          : 0
        : (index + (event.shiftKey ? -1 : 1) + targets.length) % targets.length;
    targets[next]?.focus();
  };
  dialog.addEventListener('keydown', keydown);
  return {
    update(next: { modal?: boolean } = {}) {
      options = next;
      display();
    },
    destroy() {
      unsubscribe();
      dialog.removeEventListener('keydown', keydown);
      dialog.close();
      if (previous?.isConnected) previous.focus();
    },
  };
}
