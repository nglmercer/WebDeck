import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { initI18n } from '../../framework/i18n';
import type { BootContext } from '../../framework/types';
import type { AddModalContext } from './types';
import { filterAddBrowser, wireAddModal } from './wireup';

function mount(): HTMLElement {
  document.body.innerHTML = `
    <div class="all-commands">
      <button class="dropdown-btn" dropdown-category="Spotify">Spotify</button>
      <div class="dropdown-container">
        <div class="addbutton-description"><p>Play music</p></div>
        <button class="dropdown-btn no-dropdown" dropdown-commandTag="play">Play</button>
        <div class="addbutton-description"><p>Stop music</p></div>
        <button class="dropdown-btn no-dropdown" dropdown-commandTag="stop">Stop</button>
      </div>
      <button class="dropdown-btn" dropdown-category="Display">Display</button>
      <div class="dropdown-container">
        <button class="dropdown-btn no-dropdown" dropdown-commandTag="brightness">Brightness</button>
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
    vi.useRealTimers();
    vi.unstubAllGlobals();
    document.body.innerHTML = '';
  });

  it('sends one save for rapid double submits', async () => {
    initI18n({});
    document.body.innerHTML = '<button id="t1_submit">save</button>';
    const ctx = { config: { front: {}, settings: {} }, commands: {} } as unknown as BootContext;
    const mctx: AddModalContext = {
      argModalId: 't1',
      category: 'c',
      command: 'Copy',
      parentCommand: '',
      subId: 1,
      commandValue: {},
      commandId: '/copy',
      buttonTitle: 'Copy',
    };
    wireAddModal(ctx, mctx);

    const fetchMock = vi.fn(async (url: unknown) => {
      if (String(url).includes('save_buttons_only')) {
        return { ok: true, json: async () => ({ success: true }) };
      }
      return { ok: true, json: async () => ({ front: { buttons: {} } }) };
    });
    vi.stubGlobal('fetch', fetchMock);
    vi.useFakeTimers();
    try {
      const submit = document.querySelector('#t1_submit') as HTMLElement;
      submit.click();
      submit.click();
      await vi.advanceTimersByTimeAsync(1500);
      const saves = fetchMock.mock.calls.filter(([url]) =>
        String(url).includes('save_buttons_only')
      );
      expect(saves).toHaveLength(1);
    } finally {
      vi.useRealTimers();
    }
  });
});
