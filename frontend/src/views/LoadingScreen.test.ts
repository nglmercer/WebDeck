import { mount, unmount } from 'svelte';
import { afterEach, describe, expect, it } from 'vitest';
import { html, join, type Html } from '../framework/html';
import { initI18n, text } from '../framework/i18n';
import LoadingScreen from './LoadingScreen.svelte';

/** Frozen pre-Svelte loading markup (loading.ts, since migrated). */
function legacyLoadingScreen(svgs: string[]): Html {
  return html`
    <div id="loading-screen">
      <div>
        <p id="server-disconnected" class="invisible">${text('server_disconnected')}...</p>
        <div class="loadingspinner">
          <div id="square1"></div>
          <div id="square2"></div>
          <div id="square3"></div>
          <div id="square4"></div>
          <div id="square5"></div>
        </div>
      </div>
      <div class="invisible">
        ${join(svgs.map((svg) => html`<img src="${svg}" /> `))}
      </div>
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

describe('LoadingScreen', () => {
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

  function render(svgs: string[]): HTMLElement {
    initI18n({});
    host = document.createElement('div');
    document.body.appendChild(host);
    app = mount(LoadingScreen, { target: host, props: { svgs } }) as unknown as Record<string, never>;
    return host;
  }

  it('matches the legacy markup (modulo svelte anchors)', () => {
    const svgs = ['img/a.svg', 'x"y.svg'];
    const el = render(svgs);
    const oracle = document.createElement('div');
    oracle.innerHTML = legacyLoadingScreen(svgs).value;
    expect(normalize(el.innerHTML)).toBe(normalize(oracle.innerHTML));
  });

  it('renders the spinner squares and preloads', () => {
    const el = render(['img/a.svg']);
    expect(el.querySelector('#loading-screen')).not.toBeNull();
    expect(el.querySelector('#server-disconnected')).not.toBeNull();
    expect(el.querySelectorAll('.loadingspinner > div')).toHaveLength(5);
    expect(el.querySelectorAll('#loading-screen .invisible img')).toHaveLength(1);
  });
});
