/** Native modal semantics plus deterministic Tab cycling and focus restoration. */
export function modal(dialog: HTMLDialogElement) {
  const previous = document.activeElement instanceof HTMLElement ? document.activeElement : null;
  dialog.showModal();
  const keydown = (event: KeyboardEvent) => {
    if (event.key !== 'Tab') return;
    const targets = [
      ...dialog.querySelectorAll<HTMLElement>(
        'button:not(:disabled), input:not(:disabled), select:not(:disabled), textarea:not(:disabled), a[href], [tabindex]',
      ),
    ].filter((element) => element.tabIndex >= 0 && element.getClientRects().length > 0);
    const first = targets[0],
      last = targets.at(-1);
    if (!first || !last) {
      event.preventDefault();
      dialog.focus();
      return;
    }
    if (
      event.shiftKey &&
      (document.activeElement === first || !dialog.contains(document.activeElement))
    ) {
      event.preventDefault();
      last.focus();
    } else if (
      !event.shiftKey &&
      (document.activeElement === last || !dialog.contains(document.activeElement))
    ) {
      event.preventDefault();
      first.focus();
    }
  };
  dialog.addEventListener('keydown', keydown);
  return {
    destroy() {
      dialog.removeEventListener('keydown', keydown);
      dialog.close();
      if (previous?.isConnected) previous.focus();
    },
  };
}
