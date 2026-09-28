import { mount, tick, unmount } from 'svelte';
import { afterEach, describe, expect, it } from 'vitest';
import { initI18n } from '../framework/i18n';
import type { BootContext, JsonObject } from '../framework/types';
import Grid from './Grid.svelte';

function testCtx(buttons: JsonObject, showNames = true): BootContext {
  initI18n({});
  return {
    config: { front: { buttons, show_names: showNames, names_color: '' }, settings: {} },
    commands: {},
    versions: {},
    random_bg: '',
    usage_example: {},
    langs: [],
    svgs: [],
    themes: [],
    parsed_themes: {},
    is_exe: false,
    portrait_rotate: 0,
    lang: {},
    audio_devices: {},
    dark_theme: '',
  };
}

const keyButton = { message: '/key a', name: 'a', image: 'key.png', image_size: '75%' };
const volumeButton = {
  message: '/volume + ',
  name: 'Subir el volumen',
  image: 'volume-up.svg',
  image_size: '75%',
};

describe('Grid', () => {
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

  async function render(ctx: BootContext): Promise<HTMLElement> {
    host = document.createElement('div');
    document.body.appendChild(host);
    app = mount(Grid, { target: host, props: { ctx } }) as unknown as Record<string, never>;
    // String-attrs actions land on tick.
    await tick();
    return host;
  }

  /** Grid form only (the sibling edit modal renders its own preview labels). */
  function form(host: HTMLElement, id = 'e0X0'): HTMLElement {
    return host.querySelector(`form#${id}`) as HTMLElement;
  }

  describe('key labels', () => {
    it('centers the key name inside press-key tiles instead of below them', async () => {
      const el = await render(testCtx({ index: [keyButton] }));
      expect(form(el).querySelector('.buttontext-inside')?.textContent).toBe('a');
      expect(form(el).querySelectorAll('p.buttontext:not([id])')).toHaveLength(0);
    });

    it('keeps the below-tile name for ordinary buttons', async () => {
      const el = await render(testCtx({ index: [volumeButton] }));
      expect(form(el).querySelectorAll('p.buttontext:not([id])')).toHaveLength(1);
      expect(form(el).querySelector('.buttontext-inside')).toBeNull();
    });

    it('falls back to the below-tile name when a key button has no image', async () => {
      const el = await render(testCtx({ index: [{ message: '/key a', name: 'a', image: '' }] }));
      expect(form(el).querySelectorAll('p.buttontext:not([id])')).toHaveLength(1);
      expect(form(el).querySelector('.buttontext-inside')).toBeNull();
    });

    it('shows no name anywhere when show_names is off', async () => {
      const el = await render(testCtx({ index: [keyButton] }, false));
      expect(form(el).querySelector('.buttontext-inside')).toBeNull();
      expect(form(el).querySelectorAll('p.buttontext:not([id])')).toHaveLength(0);
    });
  });

  describe('editor chrome', () => {
    it('renders badge glyphs in currentColor so badges control contrast', async () => {
      const el = await render(testCtx({ index: [volumeButton] }));
      expect(form(el).querySelector('[stroke="currentColor"]')).not.toBeNull();
      expect(form(el).querySelector('svg[fill="currentColor"]')).not.toBeNull();
    });

    it('renders void add-buttons without unitless offsets', async () => {
      const el = await render(testCtx({ index: [{ VOID: 'VOID' }] }));
      expect(el.querySelector('.void .add-button')).not.toBeNull();
      expect(el.innerHTML).not.toContain('40.3675');
    });

    it('renders the folder chip inside the form so it anchors to the tile', async () => {
      const el = await render(testCtx({ index: [{ message: '/folder docs', name: 'docs', image: '' }] }));
      expect(form(el).querySelector('.swapMode-open-folder')).not.toBeNull();
    });
  });

  describe('handlers and values', () => {
    it('keeps inline string handlers on folder buttons', async () => {
      const el = await render(testCtx({ index: [{ message: '/folder docs', name: 'docs', image: '' }] }));
      const button = form(el).querySelector('#button_e0X0') as HTMLElement;
      expect(button.getAttribute('onclick')).toBe('folder(`docs`)');
      expect(button.getAttribute('onclickhandler')).toBe('folder(`docs`)');
      expect(button.hasAttribute('type')).toBe(false);
    });

    it('renders plain submit buttons for ordinary commands', async () => {
      const el = await render(testCtx({ index: [volumeButton] }));
      const button = form(el).querySelector('#button_e0X0') as HTMLElement;
      expect(button.getAttribute('type')).toBe('submit');
      expect(button.getAttribute('onclick')).toBeNull();
    });

    it('keeps the quoted-message escaping of the hidden input', async () => {
      const el = await render(testCtx({ index: [{ message: '/say "hi"', name: 'hi', image: '' }] }));
      const input = form(el).querySelector('input.message') as HTMLInputElement;
      expect(input.value).toBe('/say &quot;hi&quot;');
    });

    it('renders usage title/value blocks for usage commands', async () => {
      const el = await render(
        testCtx({
          index: [{ message: "/usage ' CPU<|§|>' usage_dict['cpu']['x']", name: 'CPU', image: '' }],
        })
      );
      expect(form(el).querySelector('.usage-title')?.textContent?.trim()).toBe('CPU');
      expect(form(el).querySelector('.usage-value')?.textContent?.trim()).toBe('-');
    });
  });
});
