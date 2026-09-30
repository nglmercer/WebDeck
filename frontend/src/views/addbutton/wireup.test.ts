import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { initI18n } from '../../framework/i18n';
import type { BootContext } from '../../framework/types';
import type { AddModalContext } from './types';
import { filterAddBrowser, wireAddModal } from './wireup';

function mount(): HTMLElement {
  document.body.innerHTML = `
    <div class="all-commands">
      <button class="dropdown-btn" data-dropdown-category="Spotify">Spotify</button>
      <div class="dropdown-container">
        <div class="addbutton-description"><p>Play music</p></div>
        <button class="dropdown-btn no-dropdown" data-dropdown-command-tag="play">Play</button>
        <div class="addbutton-description"><p>Stop music</p></div>
        <button class="dropdown-btn no-dropdown" data-dropdown-command-tag="stop">Stop</button>
      </div>
      <button class="dropdown-btn" data-dropdown-category="Display">Display</button>
      <div class="dropdown-container">
        <button class="dropdown-btn no-dropdown" data-dropdown-command-tag="brightness">Brightness</button>
      </div>
    </div>`;
  return document.querySelector('.all-commands') as HTMLElement;
}

function displayOf(text: string): string {
  const btn = [...document.querySelectorAll('.dropdown-btn')].find(
    (b) => (b.textContent ?? '').trim() === text
  ) as HTMLElement;
  return btn.style.display;
}

describe('filterAddBrowser', () => {
  beforeEach(mount);

  it('hides non-matching leaves and reveals their container', () => {
    const root = document.querySelector('.all-commands') as HTMLElement;
    filterAddBrowser(root, 'stop');
    expect(displayOf('Play')).toBe('none');
    expect(displayOf('Stop')).toBe('');
    expect(displayOf('Brightness')).toBe('none');
    // Spotify branch stays with an opened panel; Display hides entirely.
    expect(displayOf('Spotify')).toBe('');
    expect(displayOf('Display')).toBe('none');
    const panels = [...document.querySelectorAll('.dropdown-container')] as HTMLElement[];
    expect(panels[0]!.style.display).toBe('block');
    expect(panels[1]!.style.display).toBe('none');
  });

  it('matches command tags, not just labels', () => {
    const root = document.querySelector('.all-commands') as HTMLElement;
    filterAddBrowser(root, 'bright');
    expect(displayOf('Brightness')).toBe('');
    expect(displayOf('Display')).toBe('');
  });

  it('reveals the whole subtree on a branch-label match', () => {
    const root = document.querySelector('.all-commands') as HTMLElement;
    filterAddBrowser(root, 'spotify');
    expect(displayOf('Play')).toBe('');
    expect(displayOf('Stop')).toBe('');
    expect(displayOf('Display')).toBe('none');
  });

  it('hides descriptions with their button and restores all on clear', () => {
    const root = document.querySelector('.all-commands') as HTMLElement;
    filterAddBrowser(root, 'play');
    const descs = [...document.querySelectorAll('.addbutton-description')] as HTMLElement[];
    expect(descs[0]!.style.display).toBe('');
    expect(descs[1]!.style.display).toBe('none');

    filterAddBrowser(root, '  ');
    for (const el of document.querySelectorAll('.dropdown-btn, .addbutton-description')) {
      expect((el as HTMLElement).style.display).toBe('');
    }
  });
});

describe('add submit coalescing', () => {
  afterEach(() => {
    vi.unstubAllGlobals();
    document.body.innerHTML = '';
  });

  function wireAdd(modalId: string): void {
    initI18n({});
    document.body.innerHTML = `<button id="${modalId}_submit">save</button>`;
    const ctx = { config: { front: {}, settings: {} }, commands: {} } as unknown as BootContext;
    const mctx: AddModalContext = {
      argModalId: modalId,
      category: 'c',
      command: 'Copy',
      parentCommand: '',
      subId: 1,
      commandValue: {},
      commandId: '/copy',
      buttonTitle: 'Copy',
    };
    wireAddModal(ctx, mctx);
  }

  function saveCalls(fetchMock: ReturnType<typeof vi.fn>): unknown[][] {
    return fetchMock.mock.calls.filter(([url]) => String(url).includes('save_buttons_only'));
  }

  /** Dismiss the queued alert dialog (keeps the dialog queue usable). */
  async function dismissAlert(): Promise<void> {
    await vi.waitFor(() => expect(document.querySelector('[data-testid="alert-ok"]')).not.toBeNull());
    (document.querySelector('[data-testid="alert-ok"]') as HTMLElement).click();
    await vi.waitFor(() => expect(document.querySelector('[role="alertdialog"]')).toBeNull());
  }

  it('sends one save for rapid double submits', async () => {
    wireAdd('t1');
    const fetchMock = vi.fn(async (url: unknown) => {
      if (String(url).includes('save_buttons_only')) {
        // Failure response: the save is still sent (what this counts),
        // but no modal-hide timers outlive the test environment.
        return { ok: true, headers: new Headers(), json: async () => ({ success: false }) };
      }
      return { ok: true, headers: new Headers(), json: async () => ({ front: { buttons: {} } }) };
    });
    vi.stubGlobal('fetch', fetchMock);

    const submit = document.querySelector('#t1_submit') as HTMLButtonElement;
    submit.click();
    submit.click();
    await vi.waitFor(() => expect(saveCalls(fetchMock)).toHaveLength(1));
    // Settled saves stay at one (no delayed duplicate).
    await new Promise((resolve) => setTimeout(resolve, 50));
    expect(saveCalls(fetchMock)).toHaveLength(1);
    await dismissAlert();
  });

  it('disables the submit control in flight and allows retry after failure', async () => {
    wireAdd('t2');
    let rejectSave!: (error: unknown) => void;
    const gate = new Promise<never>((_resolve, reject) => {
      rejectSave = reject;
    });
    const fetchMock = vi.fn((url: unknown) => {
      if (String(url).includes('save_buttons_only')) return gate;
      return Promise.resolve({ ok: true, headers: new Headers(), json: async () => ({ front: { buttons: {} } }) });
    });
    vi.stubGlobal('fetch', fetchMock);

    const submit = document.querySelector('#t2_submit') as HTMLButtonElement;
    submit.click();
    await vi.waitFor(() => expect(saveCalls(fetchMock)).toHaveLength(1));
    expect(submit.disabled).toBe(true);

    rejectSave(new Error('boom'));
    await vi.waitFor(() => expect(submit.disabled).toBe(false));
    // The failure stays visible (no false success): transport errors map
    // to the localized save error (i18n empty here, so the key itself).
    await vi.waitFor(() =>
      expect(document.querySelector('#wd-dialog-message')?.textContent).toBe('settings_save_error')
    );
    await dismissAlert();

    submit.click();
    await vi.waitFor(() => expect(saveCalls(fetchMock)).toHaveLength(2));
    await dismissAlert();
  });
});
