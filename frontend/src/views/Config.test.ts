import { mount, tick, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { initI18n } from '../framework/i18n';
import type { BootContext, JsonObject } from '../framework/types';
import Config from './Config.svelte';
import { send_data } from '../app/send';
vi.mock('../app/send', () => ({ send_data: vi.fn() }));
import { resolveLanguage } from './config';

function testCtx(): BootContext {
  initI18n({});
  return {
    config: {
      front: {
        buttons: { index: [] },
        show_names: true,
        names_color: 'ff0000',
        buttons_color: '#00ff00',
        height: '3',
        width: '5',
        portrait_rotate: '',
        computer_usage_reload_time: '',
        themes: ['//off.css', 'mytheme.css', 'static/css/style.css'],
        background: ['#112233', '**uploaded/v.mp4', '**uploaded/pic.png'],
        dark_theme: true,
        edit_buttons_color: false,
      },
      settings: {
        language: 'es',
        gpu_method: 'None',
        soundboard: { enabled: true, mic_input_device: 'Mic (Real)', vbcable: '' },
        ear_soundboard: false,
        spotify_api: { username: 'u', client_id: 'id', client_secret: 'secret' },
        obs: { host: 'h', port: '4455', password: 'pw' },
        optimized_usage_display: true,
        open_settings_in_integrated_browser: false,
        show_console: false,
        automatic_firewall_bypass: true,
        fix_stop_soundboard: false,
        dev_mode: false,
      },
    },
    commands: {},
    versions: { versions: [{ version: '1.2.3' }] },
    random_bg: '',
    usage_example: {},
    langs: [
      { code: 'en_US', native_name: 'English' },
      { code: 'es_ES', native_name: 'Español' },
    ],
    svgs: [],
    themes: [],
    parsed_themes: {
      'off.css': { 'theme-icon': '', 'theme-name': 'Off', 'theme-description': 'Disabled' },
      'mytheme.css': { 'theme-icon': 'icon.png', 'theme-name': 'Mine', 'theme-description': 'Custom' },
      'static/css/style.css': { 'theme-icon': '', 'theme-name': 'Default', 'theme-description': 'Built-in' },
    },
    is_exe: false,
    portrait_rotate: 0,
    lang: {},
    audio_devices: { input: ['Mic (Real)'], output: ['Speakers'] },
    dark_theme: 'dark-theme',
  };
}

describe('Config', () => {
  let host: HTMLElement | null = null;
  let app: Record<string, never> | null = null;

  beforeEach(() => {
    window.localStorage.clear();
  });

  afterEach(async () => {
    if (app) {
      await unmount(app);
      app = null;
    }
    host?.remove();
    host = null;
    document.body.innerHTML = '';
    vi.unstubAllGlobals();
  });

  async function render(ctx: BootContext = testCtx()): Promise<HTMLElement> {
    host = document.createElement('div');
    document.body.appendChild(host);
    app = mount(Config, { target: host, props: { ctx } }) as unknown as Record<string, never>;
    await tick();
    return host;
  }

  it('renders the modal shell with version links', async () => {
    const el = await render();
    expect(el.querySelector('#modal-container')).not.toBeNull();
    expect(el.querySelector('#config-form')).not.toBeNull();
    expect(el.querySelectorAll('#version')).toHaveLength(2);
    expect(el.querySelector('.modal-close svg')).not.toBeNull();
  });

  it('renders all nine settings sections', async () => {
    const el = await render();
    for (const id of [
      'settings-general',
      'settings-soundboard',
      'settings-spotify',
      'settings-obs',
      'visuals-grid',
      'visuals-theme',
      'visuals-appearance',
      'experimental-usage',
      'experimental-advanced',
    ]) {
      expect(el.querySelector(`details[data-collapse="${id}"]`), id).not.toBeNull();
    }
  });

  it('resolves the language sibling and audio devices', async () => {
    const el = await render();
    expect((el.querySelector('#language') as HTMLSelectElement).value).toBe('es_ES');
    expect((el.querySelector('#mic_input_device') as HTMLSelectElement).value).toBe('Mic (Real)');
  });

  it('fills spotify/obs credentials and soundboard switches', async () => {
    const el = await render();
    expect((el.querySelector('#spotify-username') as HTMLInputElement).value).toBe('u');
    expect((el.querySelector('#obs-PORT') as HTMLInputElement).value).toBe('4455');
    expect((el.querySelector('#toggle_soundboard') as HTMLInputElement).checked).toBe(true);
    expect((el.querySelector('#ear_soundboard') as HTMLInputElement).checked).toBe(false);
  });

  it('hides exe-only switches outside exe builds', async () => {
    const el = await render();
    expect(el.querySelector('.setting.windows-startup.invisible')).not.toBeNull();
    expect((el.querySelector('#windows-startup') as HTMLInputElement).checked).toBe(true);
  });

  it('renders grid/portrait/gpu controls with defaults', async () => {
    const el = await render();
    expect((el.querySelector('#gridsize-height') as HTMLInputElement).value).toBe('3');
    expect((el.querySelector('#portrait-rotate') as HTMLInputElement).value).toBe('90');
    expect((el.querySelector('#usage-reload-time') as HTMLInputElement).value).toBe('3000');
    expect((el.querySelector('#gpu_method') as HTMLSelectElement).value).toBe('None');
  });

  it('sets portrait rotation from the preset buttons', async () => {
    const el = await render();
    const buttons = el.querySelectorAll('#portrait-rotate-setting-container button');
    (buttons[0] as HTMLButtonElement).click();
    expect((el.querySelector('#portrait-rotate') as HTMLInputElement).value).toBe('270');
  });

  it('renders theme entries with the default marked and arrowless', async () => {
    const el = await render();
    expect(el.querySelectorAll('#disabled-themes .theme-container')).toHaveLength(1);
    expect(el.querySelectorAll('#enabled-themes .theme-container')).toHaveLength(2);
    const builtin = el.querySelector('#enabled-themes .theme-container[data-filename="static/css/style.css"]');
    expect(builtin?.hasAttribute('data-default-theme')).toBe(true);
    expect(builtin?.querySelector('.arrows-container')).toBeNull();
    expect(
      el.querySelector('#enabled-themes .theme-container[data-filename="mytheme.css"] .arrows-container')
    ).not.toBeNull();
  });

  it('renders color, video, and image backgrounds', async () => {
    const el = await render();
    const entries = el.querySelectorAll('#choose-backgrounds-container .choose-bg-element');
    expect(entries).toHaveLength(3);
    expect(entries[0]?.querySelector('.choose-bg-activate-button-checked')).not.toBeNull();
    expect(entries[1]?.querySelector('video source')?.getAttribute('src')).toBe(
      '.config/user_uploads/v.mp4'
    );
    expect(entries[2]?.querySelector('img')?.getAttribute('src')).toBe('.config/user_uploads/pic.png');
  });

  it('gives every section a distinct icon and keeps library links out of the steps', async () => {
    const el = await render();
    const icons = Array.from(
      el.querySelectorAll('details.wd-collapse .wd-collapse-icon svg')
    ).map((svg) => svg.innerHTML);
    expect(icons).toHaveLength(9);
    expect(new Set(icons).size).toBe(9);
    // Steps switch tabs; library shortcuts open panels — separate groups.
    expect(el.querySelectorAll('nav.wd2-steps .wd2-step')).toHaveLength(3);
    expect(el.querySelectorAll('.wd2-lib .wd2-lib-btn')).toHaveLength(2);
    expect(el.querySelector('nav.wd2-steps .wd2-lib-btn')).toBeNull();
  });

  it('switches the library tabs without back-button navigation', async () => {
    const el = await render();
    // No back buttons; panels are tab panes inside the form.
    expect(el.querySelector('#setting-themes-back')).toBeNull();
    expect(el.querySelector('#setting-background-back')).toBeNull();
    expect((el.querySelector('#choose-themes') as HTMLElement).style.display).not.toBe('none');
    expect(el.querySelector('.setting-category.library')?.hasAttribute('hidden')).toBe(true);

    const libBtns = el.querySelectorAll('.wd2-lib-btn') as NodeListOf<HTMLButtonElement>;
    libBtns[1]!.click();
    await tick();
    expect(el.querySelector('.setting-category.library')?.hasAttribute('hidden')).toBe(false);
    expect(el.querySelector('.setting-category.settings')?.hasAttribute('hidden')).toBe(true);
    expect(el.querySelector('#config-lib-pane-backgrounds')?.hasAttribute('hidden')).toBe(false);
    expect(el.querySelector('#config-lib-pane-themes')?.hasAttribute('hidden')).toBe(true);
    expect(libBtns[1]!.getAttribute('aria-current')).toBe('true');

    // Same-style tab strip switches panes in place.
    expect(el.querySelectorAll('.wd2-lib-tabs [role="tab"]')).toHaveLength(2);
    (el.querySelector('#config-lib-tab-themes') as HTMLButtonElement).click();
    await tick();
    expect(el.querySelector('#config-lib-pane-themes')?.hasAttribute('hidden')).toBe(false);
    expect(el.querySelector('#config-lib-pane-backgrounds')?.hasAttribute('hidden')).toBe(true);

    // Steps lead back to settings.
    (el.querySelector('nav.wd2-steps .wd2-step') as HTMLButtonElement).click();
    await tick();
    expect(el.querySelector('.setting-category.library')?.hasAttribute('hidden')).toBe(true);
    expect(el.querySelector('.setting-category.settings')?.hasAttribute('hidden')).toBe(false);
  });

  it('opens the library from the visuals shortcuts', async () => {
    const el = await render();
    (el.querySelector('#setting-themes') as HTMLButtonElement).click();
    await tick();
    expect(el.querySelector('#config-lib-pane-themes')?.hasAttribute('hidden')).toBe(false);
    expect(el.querySelector('#config-lib-pane-backgrounds')?.hasAttribute('hidden')).toBe(true);
  });

  it('renders background cards with footer, label, and live count', async () => {
    const el = await render();
    expect(el.querySelector('#bg-count')?.textContent).toBe('3');
    const entries = el.querySelectorAll('#choose-backgrounds-container .choose-bg-element');
    for (const entry of Array.from(entries)) {
      expect(entry.querySelector('.choose-bg-foot')).not.toBeNull();
      expect(entry.querySelector('.choose-bg-label')?.textContent?.trim()).not.toBe('');
      expect(entry.querySelector('.choose-bg-buttons')).not.toBeNull();
    }
    expect(entries[0]?.querySelector('.choose-bg-swatch')).not.toBeNull();
    expect(entries[2]?.querySelector('.choose-bg-thumb img')).not.toBeNull();
    // Activate is a real toggle button; upload uses the shared file row.
    const toggle = entries[0]?.querySelector('.choose-bg-activate-button');
    expect(toggle?.tagName).toBe('BUTTON');
    expect(toggle?.getAttribute('aria-pressed')).toBe('true');
    expect(el.querySelector('#create-image-bg.wd2-dropfile-input')).not.toBeNull();
  });

  it('switches the add-background tabs while keeping both panes mounted', async () => {
    const el = await render();
    const colorTab = el.querySelector('#bg-add-tab-color') as HTMLButtonElement;
    const fileTab = el.querySelector('#bg-add-tab-file') as HTMLButtonElement;
    const colorPane = el.querySelector('#bg-add-pane-color') as HTMLElement;
    const filePane = el.querySelector('#bg-add-pane-file') as HTMLElement;
    // Color composer first; file pane mounted but hidden.
    expect(colorTab.getAttribute('aria-selected')).toBe('true');
    expect(colorPane.hasAttribute('hidden')).toBe(false);
    expect(filePane.hasAttribute('hidden')).toBe(true);

    fileTab.click();
    await tick();
    expect(fileTab.getAttribute('aria-selected')).toBe('true');
    expect(colorTab.getAttribute('aria-selected')).toBe('false');
    expect(filePane.hasAttribute('hidden')).toBe(false);
    expect(colorPane.hasAttribute('hidden')).toBe(true);

    // Hidden panes stay in the DOM: the legacy wiring hooks survive.
    expect(el.querySelector('#background-color-hex')).not.toBeNull();
    expect(el.querySelector('#create-color-bg')).not.toBeNull();
    expect(el.querySelector('#create-image-bg.wd2-dropfile-input')).not.toBeNull();
  });

  it('normalizes color settings and theme reprs', async () => {
    const el = await render();
    expect((el.querySelector('#names-color-hex') as HTMLInputElement).value).toBe('#ff0000');
    expect((el.querySelector('#buttons-color-hex') as HTMLInputElement).value).toBe('#00ff00');
    expect((el.querySelector('#choose-themes-handler') as HTMLInputElement).value).toContain(
      '"mytheme.css"'
    );
  });

  it('enables a theme to the top and serializes the list', async () => {
    const el = await render();
    const handler = () => (el.querySelector('#choose-themes-handler') as HTMLInputElement).value;
    expect(JSON.parse(handler())).toEqual(['//off.css', 'mytheme.css', 'static/css/style.css']);

    (el.querySelector('#disabled-themes .enable-theme-hitbox') as HTMLElement).click();
    await tick();

    expect(el.querySelectorAll('#disabled-themes .theme-container')).toHaveLength(0);
    expect(
      [...el.querySelectorAll('#enabled-themes .theme-container')].map((e) =>
        e.getAttribute('data-filename')
      )
    ).toEqual(['off.css', 'mytheme.css', 'static/css/style.css']);
    expect(JSON.parse(handler())).toEqual(['off.css', 'mytheme.css', 'static/css/style.css']);
  });

  it('disables a theme back with the // prefix', async () => {
    const el = await render();
    (
      el.querySelector(
        '#enabled-themes .theme-container[data-filename="mytheme.css"] .disable-theme-hitbox'
      ) as HTMLElement
    ).click();
    await tick();

    expect(
      [...el.querySelectorAll('#disabled-themes .theme-container')].map((e) =>
        e.getAttribute('data-filename')
      )
    ).toEqual(['//mytheme.css', '//off.css']);
    expect(
      JSON.parse((el.querySelector('#choose-themes-handler') as HTMLInputElement).value)
    ).toEqual(['//mytheme.css', '//off.css', 'static/css/style.css']);
  });

  it('reorders themes with the arrows but never across the default', async () => {
    const ctx = testCtx();
    (ctx.config as JsonObject)['front'] = {
      ...((ctx.config as JsonObject)['front'] as JsonObject),
      themes: ['a.css', 'b.css', 'c.css', 'static/css/style.css'],
    };
    const el = await render(ctx);
    const order = () =>
      [...el.querySelectorAll('#enabled-themes .theme-container')].map((e) =>
        e.getAttribute('data-filename')
      );
    const up = (file: string) =>
      el.querySelector(
        `#enabled-themes .theme-container[data-filename="${file}"] .arrow-up-hitbox`
      ) as HTMLElement;
    const down = (file: string) =>
      el.querySelector(
        `#enabled-themes .theme-container[data-filename="${file}"] .arrow-down-hitbox`
      ) as HTMLElement;

    up('b.css').click();
    await tick();
    expect(order()).toEqual(['b.css', 'a.css', 'c.css', 'static/css/style.css']);
    expect(
      JSON.parse((el.querySelector('#choose-themes-handler') as HTMLInputElement).value)
    ).toEqual(['b.css', 'a.css', 'c.css', 'static/css/style.css']);

    // First row cannot move up; nothing changes.
    up('b.css').click();
    await tick();
    expect(order()).toEqual(['b.css', 'a.css', 'c.css', 'static/css/style.css']);

    // Last row cannot move down past the default theme.
    down('c.css').click();
    await tick();
    expect(order()).toEqual(['b.css', 'a.css', 'c.css', 'static/css/style.css']);

    down('a.css').click();
    await tick();
    expect(order()).toEqual(['b.css', 'c.css', 'a.css', 'static/css/style.css']);
  });

  it('reveals row arrows on hover, except on the default theme', async () => {
    const el = await render();
    const row = el.querySelector(
      '#enabled-themes .theme-container[data-filename="mytheme.css"]'
    ) as HTMLElement;
    const arrows = () => row.querySelector('.arrows-container') as HTMLElement;
    expect(arrows().classList.contains('invisible')).toBe(true);

    row.dispatchEvent(new MouseEvent('mouseenter', { bubbles: true }));
    await tick();
    expect(arrows().classList.contains('invisible')).toBe(false);
    expect(row.querySelector('.disable-theme')?.classList.contains('invisible')).toBe(false);

    row.dispatchEvent(new MouseEvent('mouseleave', { bubbles: true }));
    await tick();
    expect(arrows().classList.contains('invisible')).toBe(true);

    // Disabled rows show the toggle glyph but never the reorder arrows.
    const off = el.querySelector('#disabled-themes .theme-container') as HTMLElement;
    off.dispatchEvent(new MouseEvent('mouseenter', { bubbles: true }));
    await tick();
    expect(off.querySelector('.enable-theme')?.classList.contains('invisible')).toBe(false);
    expect(off.querySelector('.arrows-container')?.classList.contains('invisible')).toBe(true);

    // The default theme has no toggle or arrows at all.
    const builtin = el.querySelector(
      '#enabled-themes .theme-container[data-filename="static/css/style.css"]'
    ) as HTMLElement;
    expect(builtin.querySelector('.disable-theme-hitbox')).toBeNull();
    expect(builtin.querySelector('.arrows-container')).toBeNull();
  });

  it('toggles backgrounds off and on with the // prefix', async () => {
    const el = await render();
    const handler = () => (el.querySelector('#choose-background-handler') as HTMLInputElement).value;
    expect(JSON.parse(handler())).toEqual(['#112233', '**uploaded/v.mp4', '**uploaded/pic.png']);

    const toggle = el.querySelector(
      '#choose-backgrounds-container .choose-bg-element .choose-bg-activate-button'
    ) as HTMLButtonElement;
    toggle.click();
    await tick();
    expect(JSON.parse(handler())).toEqual(['//#112233', '**uploaded/v.mp4', '**uploaded/pic.png']);
    expect(toggle.classList.contains('choose-bg-activate-button-checked')).toBe(false);
    expect(toggle.getAttribute('aria-pressed')).toBe('false');

    toggle.click();
    await tick();
    expect(JSON.parse(handler())).toEqual(['#112233', '**uploaded/v.mp4', '**uploaded/pic.png']);
    expect(toggle.getAttribute('aria-pressed')).toBe('true');
  });

  it('keeps the last active background on', async () => {
    const el = await render();
    const toggles = [
      ...el.querySelectorAll(
        '#choose-backgrounds-container .choose-bg-element .choose-bg-activate-button'
      ),
    ] as HTMLButtonElement[];
    toggles[0]!.click();
    await tick();
    toggles[1]!.click();
    await tick();

    // Two off: the third toggle is refused.
    toggles[2]!.click();
    await tick();
    expect(JSON.parse((el.querySelector('#choose-background-handler') as HTMLInputElement).value)).toEqual(
      ['//#112233', '//**uploaded/v.mp4', '**uploaded/pic.png']
    );
    expect(toggles[2]!.getAttribute('aria-pressed')).toBe('true');
  });

  it('deletes backgrounds except the last one and the last active one', async () => {
    const el = await render();
    const handler = () => (el.querySelector('#choose-background-handler') as HTMLInputElement).value;
    const cards = () => el.querySelectorAll('#choose-backgrounds-container .choose-bg-element');
    const trashOf = (index: number) =>
      cards()[index]!.querySelector('.choose-bg-delete-button') as unknown as Element;
    const trashClick = (index: number) =>
      trashOf(index).dispatchEvent(new MouseEvent('click', { bubbles: true }));

    trashClick(0);
    await tick();
    expect(cards()).toHaveLength(2);
    expect(el.querySelector('#bg-count')?.textContent).toBe('2');
    expect(JSON.parse(handler())).toEqual(['**uploaded/v.mp4', '**uploaded/pic.png']);

    trashClick(0);
    await tick();
    expect(cards()).toHaveLength(1);
    // The last background cannot be deleted.
    trashClick(0);
    await tick();
    expect(cards()).toHaveLength(1);
    expect(JSON.parse(handler())).toEqual(['**uploaded/pic.png']);
  });

  it('deletes a disabled background but never the last active one', async () => {
    const el = await render();
    const toggles = [
      ...el.querySelectorAll(
        '#choose-backgrounds-container .choose-bg-element .choose-bg-activate-button'
      ),
    ] as HTMLButtonElement[];
    toggles[0]!.click();
    await tick();
    toggles[1]!.click();
    await tick();

    const trash = (index: number) =>
      el.querySelectorAll('#choose-backgrounds-container .choose-bg-element')[index]!.querySelector(
        '.choose-bg-delete-button'
      ) as unknown as Element;
    const trashClick = (index: number) =>
      trash(index).dispatchEvent(new MouseEvent('click', { bubbles: true }));
    // Last active card: deletion refused.
    trashClick(2);
    await tick();
    expect(el.querySelectorAll('#choose-backgrounds-container .choose-bg-element')).toHaveLength(3);
    // Disabled cards delete freely.
    trashClick(0);
    await tick();
    expect(el.querySelectorAll('#choose-backgrounds-container .choose-bg-element')).toHaveLength(2);
    expect(JSON.parse((el.querySelector('#choose-background-handler') as HTMLInputElement).value)).toEqual(
      ['//**uploaded/v.mp4', '**uploaded/pic.png']
    );
  });

  it('adds a color background from the composer', async () => {
    const el = await render();
    const hex = el.querySelector('#background-color-hex') as HTMLInputElement;
    hex.value = '#aabbcc';
    hex.dispatchEvent(new Event('input', { bubbles: true }));
    await tick();
    (el.querySelector('#create-color-bg') as HTMLButtonElement).click();
    await tick();

    const cards = el.querySelectorAll('#choose-backgrounds-container .choose-bg-element');
    expect(cards).toHaveLength(4);
    expect(el.querySelector('#bg-count')?.textContent).toBe('4');
    expect(cards[3]?.getAttribute('data-background')).toBe('#aabbcc');
    expect(cards[3]?.querySelector('.choose-bg-label')?.textContent).toContain('#aabbcc');
    expect(
      cards[3]
        ?.querySelector('.choose-bg-activate-button')
        ?.classList.contains('choose-bg-activate-button-checked')
    ).toBe(true);
    expect(JSON.parse((el.querySelector('#choose-background-handler') as HTMLInputElement).value)).toEqual(
      ['#112233', '**uploaded/v.mp4', '**uploaded/pic.png', '#aabbcc']
    );
  });

  it('ignores the color composer when empty', async () => {
    const el = await render();
    (el.querySelector('#create-color-bg') as HTMLButtonElement).click();
    await tick();
    expect(el.querySelectorAll('#choose-backgrounds-container .choose-bg-element')).toHaveLength(3);
  });

  it('uploads an image background through the file composer', async () => {
    const el = await render();
    vi.stubGlobal(
      'fetch',
      vi.fn(async () => ({ ok: true }))
    );
    const input = el.querySelector('#create-image-bg') as HTMLInputElement;
    Object.defineProperty(input, 'files', {
      value: [new File(['x'], 'wall.png', { type: 'image/png' })],
      configurable: true,
    });
    input.dispatchEvent(new Event('change', { bubbles: true }));
    await vi.waitFor(() =>
      expect(el.querySelectorAll('#choose-backgrounds-container .choose-bg-element')).toHaveLength(4)
    );

    const cards = el.querySelectorAll('#choose-backgrounds-container .choose-bg-element');
    expect(cards[3]?.getAttribute('data-background')).toBe('**uploaded/wall.png');
    expect(cards[3]?.querySelector('.choose-bg-label')?.textContent).toBe('wall.png');
    expect(cards[3]?.querySelector('.choose-bg-thumb img')?.getAttribute('src')).toBe(
      '.config/user_uploads/wall.png'
    );
    expect(el.querySelector('#bg-count')?.textContent).toBe('4');
    expect(JSON.parse((el.querySelector('#choose-background-handler') as HTMLInputElement).value)).toEqual(
      ['#112233', '**uploaded/v.mp4', '**uploaded/pic.png', '**uploaded/wall.png']
    );
  });

  it('keeps the list when a background upload fails', async () => {
    const el = await render();
    vi.stubGlobal(
      'fetch',
      vi.fn(async () => {
        throw new Error('network down');
      })
    );
    const errors: unknown[][] = [];
    const origError = console.error;
    console.error = (...args: unknown[]) => {
      errors.push(args);
    };
    try {
      const input = el.querySelector('#create-image-bg') as HTMLInputElement;
      Object.defineProperty(input, 'files', {
        value: [new File(['x'], 'wall.png', { type: 'image/png' })],
        configurable: true,
      });
      input.dispatchEvent(new Event('change', { bubbles: true }));
      await vi.waitFor(() => expect(errors.length).toBeGreaterThan(0));
      expect(el.querySelectorAll('#choose-backgrounds-container .choose-bg-element')).toHaveLength(3);
      expect(el.querySelector('#bg-count')?.textContent).toBe('3');
    } finally {
      console.error = origError;
    }
  });

  it('sends the firewall bypass through the typed command API', async () => {
    const send = vi.mocked(send_data);
    send.mockClear();
    const el = await render();
    (el.querySelector('#authorize_windows_firewall') as HTMLButtonElement).click();
    expect(send).toHaveBeenCalledWith('/bypass-windows-firewall');
  });
});

function langs(): JsonObject[] {
  return [{ code: 'en_US' }, { code: 'es_ES' }, { code: 'fr_FR' }];
}

describe('resolveLanguage', () => {
  it('matches exact, short, and sibling locales', () => {
    expect(resolveLanguage('es_ES', langs())).toBe('es_ES');
    expect(resolveLanguage('ES_es', langs())).toBe('es_ES');
    expect(resolveLanguage('es', langs())).toBe('es_ES');
    expect(resolveLanguage('es_PE', langs())).toBe('es_ES');
    expect(resolveLanguage('es-PE', langs())).toBe('es_ES');
  });

  it('falls back to en_US without a linguistic match', () => {
    expect(resolveLanguage('xx_YY', langs())).toBe('en_US');
    expect(resolveLanguage('', langs())).toBe('en_US');
  });
});
