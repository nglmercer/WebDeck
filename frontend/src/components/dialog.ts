import { mount, unmount } from 'svelte';
import { text } from '../framework/i18n';
import Dialog from './Dialog.svelte';

/**
 * Blocking alert/confirm dialogs (replaces `window.alert`/`window.confirm`).
 *
 * The dialog mounts into a host `div` on `document.body`, so app refreshes
 * (which re-render `#app`) never tear it down mid-prompt. Calls are served
 * from a serial queue: native dialogs blocked the thread, so stacked
 * prompts never overlapped — the chain keeps that guarantee.
 */

export interface AlertOptions {
  /** Defaults to `generic_accept`. */
  confirmLabel?: string;
}

export interface ConfirmOptions {
  /** Defaults to `generic_accept`. */
  confirmLabel?: string;
  /** Defaults to `generic_cancel`. */
  cancelLabel?: string;
  /** Red primary button for destructive confirms. */
  danger?: boolean;
}

interface DialogProps {
  variant: 'alert' | 'confirm';
  message: string;
  confirmLabel: string;
  cancelLabel: string;
  danger: boolean;
}

let tail: Promise<unknown> = Promise.resolve();

function enqueue<T>(run: () => Promise<T>): Promise<T> {
  const next = tail.then(run, run);
  tail = next.then(
    () => undefined,
    () => undefined
  );
  return next;
}

function openDialog(props: DialogProps): Promise<boolean> {
  return new Promise((resolve) => {
    const host = document.createElement('div');
    document.body.appendChild(host);
    const app = mount(Dialog, {
      target: host,
      props: {
        ...props,
        onResolve: (confirmed: boolean) => {
          // Resolve after teardown: the queue serves the next prompt only
          // once this host is fully gone, so dialogs never overlap.
          void unmount(app).then(() => {
            host.remove();
            resolve(confirmed);
          });
        },
      },
    });
  });
}

/** Informational acknowledgment; resolves once dismissed. */
export function showAlert(message: string, opts: AlertOptions = {}): Promise<void> {
  const confirmLabel = opts.confirmLabel ?? text('generic_accept');
  return enqueue(() =>
    openDialog({ variant: 'alert', message, confirmLabel, cancelLabel: '', danger: false }).then(
      () => undefined
    )
  );
}

/** Choice prompt; resolves true on confirm, false on cancel/Escape/backdrop. */
export function showConfirm(message: string, opts: ConfirmOptions = {}): Promise<boolean> {
  const confirmLabel = opts.confirmLabel ?? text('generic_accept');
  const cancelLabel = opts.cancelLabel ?? text('generic_cancel');
  return enqueue(() =>
    openDialog({
      variant: 'confirm',
      message,
      confirmLabel,
      cancelLabel,
      danger: opts.danger ?? false,
    })
  );
}
