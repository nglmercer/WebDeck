import { mount, tick, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { initI18n } from '../framework/i18n';
import type { BootContext, JsonObject } from '../framework/types';
import Config from './Config.svelte';
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
    const builtin = el.querySelector('#enabled-themes .theme-container[filename="static/css/style.css"]');
    expect(builtin?.hasAttribute('defaulttheme')).toBe(true);
    expect(builtin?.querySelector('.arrows-container')).toBeNull();
    expect(
      el.querySelector('#enabled-themes .theme-container[filename="mytheme.css"] .arrows-container')
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

  it('normalizes color settings and theme reprs', async () => {
    const el = await render();
    expect((el.querySelector('#names-color-hex') as HTMLInputElement).value).toBe('#ff0000');
    expect((el.querySelector('#buttons-color-hex') as HTMLInputElement).value).toBe('#00ff00');
    expect((el.querySelector('#choose-themes-handler') as HTMLInputElement).value).toContain(
      '"mytheme.css"'
    );
  });

  it('sends the firewall bypass through the global', async () => {
    const send = vi.fn();
    window.send_data = send;
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
