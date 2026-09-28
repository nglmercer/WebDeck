import { tick } from 'svelte';
import { afterEach, describe, expect, it } from 'vitest';
import { initI18n } from '../framework/i18n';
import { showAlert, showConfirm } from './dialog';

function okButton(): HTMLButtonElement | null {
  return document.querySelector('[data-testid="alert-ok"], [data-testid="confirm-ok"]');
}

function click(el: Element | null): void {
  el?.dispatchEvent(new MouseEvent('click', { bubbles: true }));
}

/** Queued dialogs mount in microtasks; flush before asserting or clicking. */
async function shown(): Promise<void> {
  await tick();
}

async function waitForMessage(expected: string): Promise<void> {
  for (let i = 0; i < 100; i++) {
    if (document.querySelector('.wd-dialog-message')?.textContent === expected) return;
    await new Promise((resolve) => setTimeout(resolve, 5));
  }
  throw new Error(`dialog never showed message: ${expected}`);
}

describe('showAlert/showConfirm', () => {
  afterEach(() => {
    document.body.innerHTML = '';
  });

  it('renders an alert with message and a single button', async () => {
    initI18n({});
    const done = showAlert('saved ok');
    await shown();
    const dialog = document.querySelector('.wd-dialog');
    expect(dialog?.getAttribute('role')).toBe('alertdialog');
    expect(dialog?.getAttribute('aria-modal')).toBe('true');
    expect(document.querySelector('.wd-dialog-message')?.textContent).toBe('saved ok');
    expect(document.querySelector('[data-testid="alert-ok"]')?.textContent).toBe(
      'generic_accept'
    );
    expect(document.querySelector('[data-testid="confirm-cancel"]')).toBeNull();
    click(okButton());
    await done;
  });

  it('resolves confirm true on OK and false on cancel', async () => {
    initI18n({});
    const accepted = showConfirm('delete?');
    await shown();
    click(document.querySelector('[data-testid="confirm-ok"]'));
    await expect(accepted).resolves.toBe(true);

    const declined = showConfirm('delete?');
    await shown();
    expect(document.querySelector('[data-testid="confirm-cancel"]')?.textContent).toBe(
      'generic_cancel'
    );
    click(document.querySelector('[data-testid="confirm-cancel"]'));
    await expect(declined).resolves.toBe(false);
  });

  it('treats Escape and backdrop as cancel for confirms', async () => {
    initI18n({});
    const viaEscape = showConfirm('delete?');
    await shown();
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
    await expect(viaEscape).resolves.toBe(false);

    const viaBackdrop = showConfirm('delete?');
    await shown();
    const overlay = document.querySelector('.wd-dialog-overlay') as HTMLElement;
    overlay.dispatchEvent(new MouseEvent('click', { bubbles: true }));
    await expect(viaBackdrop).resolves.toBe(false);
  });

  it('dismisses alerts on Escape and removes the host from the body', async () => {
    initI18n({});
    const done = showAlert('saved ok');
    await shown();
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
    await done;
    await shown();
    expect(document.querySelector('.wd-dialog-overlay')).toBeNull();
  });

  it('queues prompts so only one dialog shows at a time', async () => {
    initI18n({});
    const first = showAlert('first');
    const second = showAlert('second');
    await shown();
    expect(document.querySelectorAll('.wd-dialog-overlay')).toHaveLength(1);
    await waitForMessage('first');
    click(okButton());
    await first;
    await waitForMessage('second');
    expect(document.querySelectorAll('.wd-dialog-overlay')).toHaveLength(1);
    click(okButton());
    await second;
    await shown();
    expect(document.querySelector('.wd-dialog-overlay')).toBeNull();
  });
});
