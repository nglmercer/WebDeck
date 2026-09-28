import { mount, unmount } from 'svelte';
import { afterEach, describe, expect, it } from 'vitest';
import { html, type Html } from '../../framework/html';
import { initI18n, text } from '../../framework/i18n';
import EditorBar from './EditorBar.svelte';

/** Frozen pre-Svelte editor bar (editor/bar.ts, since migrated). */
function legacyEditorBar(): Html {
  return html`
    <div id="EditorButtons" style="display: none;">
      <span id="swapHint" style="display: none;">${text('swap_hint')}</span>
      <button class="button" id="SaveExitEditorButton">
        <img src="static/img/save.svg" width="20" height="20" id="EditorButtonLogo" />
        [E] ${text('save_and_exit')}
      </button>
      <button class="button" id="exitEditorButton"> [Q] ${text('quit_without_saving')} </button>

      <button class="button" id="swapEditorButton">
        <img src="static/img/swap.png" width="20" height="20" id="EditorButtonLogo" />
        [S] ${text('swap_buttons')}
      </button>
    </div>
  `;
}

/** Strip Svelte anchor comments + whitespace for comparison. */
function normalize(out: string): string {
  return out
    .replace(/<!--[\s\S]*?-->/g, '')
    .replace(/\s+/g, ' ')
    .replace(/> /g, '>')
    .replace(/ </g, '<')
    .trim();
}

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

  it('matches the legacy markup (modulo svelte anchors)', () => {
    const el = render();
    const oracle = document.createElement('div');
    oracle.innerHTML = legacyEditorBar().value;
    expect(normalize(el.innerHTML)).toBe(normalize(oracle.innerHTML));
  });

  it('renders the swap hint slot and editor buttons', () => {
    const el = render();
    expect(el.querySelector('#swapHint')).not.toBeNull();
    expect(el.querySelector('#SaveExitEditorButton')).not.toBeNull();
    expect(el.querySelector('#exitEditorButton')).not.toBeNull();
    expect(el.querySelector('#swapEditorButton')).not.toBeNull();
  });
});
