import { mount, tick, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { initI18n } from '../framework/i18n';
import type { BootContext, JsonObject } from '../framework/types';
import Shell from './Shell.svelte';
import { applyHead } from './shell';

function testCtx(themes: string[]): BootContext {
  return {
    config: { front: { themes }, settings: {} },
    commands: {},
    versions: {},
    random_bg: '',
    usage_example: {},
    langs: [],
    svgs: [],
    themes: [],
    parsed_themes: {},
    is_exe: true,
    portrait_rotate: 0,
    lang: {},
    audio_devices: {},
    dark_theme: '',
  };
}

function stylesheetHrefs(): string[] {
  return [...document.querySelectorAll('link[rel="stylesheet"]')].map(
    (link) => (link as HTMLLinkElement).getAttribute('href') ?? ''
  );
}

beforeEach(() => {
  document.head.innerHTML = '';
});

describe('applyHead themes', () => {
  it('links the built-in stylesheet directly instead of prefixing .config/themes', async () => {
    applyHead(testCtx(['static/css/style.css']));
    expect(stylesheetHrefs()).toEqual(['/static/css/style.css']);
  });

  it('keeps prefixing user themes and skipping commented entries', async () => {
    applyHead(testCtx(['mytheme.css', '//static/css/style.css']));
    expect(stylesheetHrefs()).toEqual(['.config/themes/mytheme.css']);
  });

  it('loads the base theme before user overrides (cascade order)', async () => {
    applyHead(testCtx(['mytheme.css', 'static/css/style.css']));
    expect(stylesheetHrefs()).toEqual(['/static/css/style.css', '.config/themes/mytheme.css']);
  });
});

function shellCtx(front: JsonObject, settings: JsonObject = {}, randomBg = ''): BootContext {
  initI18n({});
  return {
    config: { front, settings },
    commands: {},
    versions: {},
    random_bg: randomBg,
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

describe('Shell', () => {
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
    app = mount(Shell, { target: host, props: { ctx } }) as unknown as Record<string, never>;
    // String-attrs actions land on tick.
    await tick();
    return host;
  }

  it('renders a background video only for .mp4 bgs', async () => {
    const el = await render(shellCtx({ buttons: {} }, {}, 'clip.mp4'));
    const source = el.querySelector('.background-video video source');
    expect(source?.getAttribute('src')).toBe('.config/user_uploads/clip.mp4');
    expect(source?.getAttribute('type')).toBe('video/mp4');

    const plain = await render(shellCtx({ buttons: {} }, {}, 'red'));
    expect(plain.querySelector('.background-video')).toBeNull();
  });

  it('strips upload markers from video paths', async () => {
    const el = await render(shellCtx({ buttons: {} }, {}, '**uploaded/x.mp4'));
    expect(el.querySelector('.background-video video source')?.getAttribute('src')).toBe(
      '.config/user_uploads/x.mp4'
    );
  });

  it('renders the debug console only when enabled', async () => {
    const ctx = shellCtx({ buttons: {} }, { show_console: true });
    ctx.dark_theme = 'dark-theme';
    const el = await render(ctx);
    expect(el.querySelector('form.form input.message.dark-theme')).not.toBeNull();

    const off = await render(shellCtx({ buttons: {} }, {}));
    expect(off.querySelector('form.form')).toBeNull();
  });

  it('emits dynamic CSS for rotation, button colors, and per-button keyframes', async () => {
    const el = await render(
      shellCtx({
        buttons: { index: [{ background_color: '#00ff00' }] },
        portrait_rotate: '90',
        edit_buttons_color: true,
        buttons_color: '#ff0000',
        'buttons-color': '#ff0000',
      })
    );
    const css = el.querySelector('style')?.textContent ?? '';
    expect(css).toContain('rotate(90deg)');
    expect(css).toContain('.wd_button');
    expect(css).toContain('animation-ff0000');
    expect(css).toContain('@keyframes animation-00ff00');
    expect(css).toContain('.button-00ff00');
  });

  it('renders one folder tab per folder with string handlers', async () => {
    const el = await render(shellCtx({ buttons: { index: [], spotify: [] } }));
    const tabs = el.querySelectorAll('#EditorButtons-Folders button.EditorButtons-Folder');
    expect(tabs).toHaveLength(2);
    expect(tabs[0]?.getAttribute('onclick')).toContain('folder(');
    expect(tabs[0]?.querySelector('svg.delete-icon')).not.toBeNull();
  });

  it('keeps the upstream double-escaping quirk for quoted folder ids', async () => {
    const el = await render(shellCtx({ buttons: { 'a"b': [] } }));
    const tab = el.querySelector('#EditorButtons-Folders button.EditorButtons-Folder');
    // Upstream replaces '"' then autoescapes: the parsed attribute holds &quot;.
    expect(tab?.getAttribute('onclick')).toContain('&quot;');
    expect(tab?.textContent).toContain('a"b');
  });
});
