import { mount, unmount } from 'svelte';
import { afterEach, describe, expect, it } from 'vitest';
import { initI18n } from '../../framework/i18n';
import { swapButtonLabel } from './display';
import EditorBar from './EditorBar.svelte';

describe('EditorBar', () => {
  let host: HTMLElement | null = null;
  let app: Record<string, never> | null = null;

  afterEach(async () => {
    if (app) {
      await unmount(app);
      app = null;
    }
    host?.remove();
    host = null;
    document.body.innerHTML = '';
  });

  function render(): HTMLElement {
    initI18n({});
    host = document.createElement('div');
    document.body.appendChild(host);
    app = mount(EditorBar, { target: host }) as unknown as Record<string, never>;
    return host;
  }

  it('renders the swap hint slot and editor buttons', () => {
    const el = render();
    expect(el.querySelector('#swapHint')).not.toBeNull();
    expect(el.querySelector('#SaveExitEditorButton')).not.toBeNull();
    expect(el.querySelector('#exitEditorButton')).not.toBeNull();
    expect(el.querySelector('#swapEditorButton')).not.toBeNull();
  });

  it('shows icon, label, and shortcut chip per action', () => {
    const el = render();
    for (const [id, key, shortcut] of [
      ['SaveExitEditorButton', 'save_and_exit', 'E'],
      ['exitEditorButton', 'quit_without_saving', 'Q'],
      ['swapEditorButton', 'swap_buttons', 'S'],
    ] as const) {
      const button = el.querySelector(`#${id}`)!;
      expect(button.querySelector('img.editor-btn-icon')).not.toBeNull();
      expect(button.querySelector('.editor-btn-label')?.textContent).toBe(key);
      expect(button.querySelector('kbd.editor-kbd')?.textContent).toBe(shortcut);
      expect(button.getAttribute('title')).toBe(`${key} (${shortcut})`);
      expect(button.getAttribute('aria-keyshortcuts')).toBe(shortcut.toLowerCase());
    }
  });

  it('exposes the swap label slot for mode toggles', () => {
    const el = render();
    const label = el.querySelector('#swapEditorButton #swapEditorLabel');
    expect(label?.textContent).toBe('swap_buttons');
    expect(swapButtonLabel()?.textContent).toBe('swap_buttons');
  });

  it('uses no duplicate ids', () => {
    const el = render();
    const ids = [...el.querySelectorAll('[id]')].map((node) => node.id);
    expect(new Set(ids).size).toBe(ids.length);
  });
});
