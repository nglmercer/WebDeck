import { mount, unmount } from 'svelte';
import { afterEach, describe, expect, it } from 'vitest';
import { initI18n } from '../framework/i18n';
import type { BootContext, JsonObject } from '../framework/types';
import App from './App.svelte';

function testCtx(buttons: JsonObject, darkTheme = ''): BootContext {
  initI18n({});
  return {
    config: { front: { buttons, show_names: true, names_color: '' }, settings: {} },
    commands: { Calc: { add: { command: '/add', args: [] } } },
    versions: {},
    random_bg: '',
    usage_example: {},
    langs: [],
    svgs: ['img/a.svg', 'x"y.svg'],
    themes: [],
    parsed_themes: {},
    is_exe: false,
    portrait_rotate: 0,
    lang: {},
    audio_devices: {},
    dark_theme: darkTheme,
  };
}

const volumeButton = {
  message: '/volume + ',
  name: 'Subir el volumen',
  image: 'volume-up.svg',
  image_size: '75%',
};

describe('App shell', () => {
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

  function render(ctx: BootContext): HTMLElement {
    host = document.createElement('div');
    document.body.appendChild(host);
    app = mount(App, { target: host, props: { ctx } }) as unknown as Record<string, never>;
    return host;
  }

  it('composes every page region', () => {
    const el = render(testCtx({ index: [volumeButton] }));
    expect(el.querySelector('#loading-screen')).not.toBeNull();
    expect(el.querySelector('#EditorButtons-Folders')).not.toBeNull();
    expect(el.querySelector('#deck-scale')).not.toBeNull();
    expect(el.querySelector('#EditorButtons')).not.toBeNull();
    expect(el.querySelector('#config-form')).not.toBeNull();
    expect(el.querySelector('.addbutton-modal-container')).not.toBeNull();
    // Shell dynamic CSS + portraits/tiles keyframes.
    expect(el.querySelector('style')).not.toBeNull();
  });

  it('keeps tour-critical ids and classes stable', () => {
    const el = render(testCtx({ index: [volumeButton] }));
    expect(el.querySelector('#loading-screen')).not.toBeNull();
    expect(el.querySelector('#deck-scale')).not.toBeNull();
    expect(el.querySelector('#button_e0X0')).not.toBeNull();
    expect(el.querySelector('#EditorButtons-Folders')).not.toBeNull();
    expect(el.querySelectorAll('.loadingspinner > div')).toHaveLength(5);
    const preloads = el.querySelectorAll('#loading-screen .invisible img');
    expect(preloads).toHaveLength(2);
    expect(preloads[1]?.getAttribute('src')).toBe('x"y.svg');
  });

  it('renders one tile per configured button', () => {
    const el = render(testCtx({ index: [volumeButton, volumeButton] }));
    expect(el.querySelector('#button_e0X0')).not.toBeNull();
    expect(el.querySelector('#button_e0X1')).not.toBeNull();
  });

  it('plumbs the theme class through every modal region', () => {
    const dark = render(testCtx({ index: [volumeButton] }, 'dark-theme'));
    for (const selector of [
      '.modal-container.dark-theme',
      '.addbutton-modal-container.dark-theme',
      '.addbutton-modal-container-args.dark-theme',
      '.editbutton-modal-container.dark-theme',
    ]) {
      expect(dark.querySelector(selector), selector).not.toBeNull();
    }
  });

  it('leaves modal regions unthemed without a theme', () => {
    const light = render(testCtx({ index: [volumeButton] }));
    expect(light.querySelector('.dark-theme')).toBeNull();
  });
});
