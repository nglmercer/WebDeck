import { mount, unmount } from 'svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import TextField from './TextField.svelte';

describe('TextField', () => {
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
    vi.unstubAllGlobals();
  });

  function render(props: Record<string, unknown>): HTMLElement {
    host = document.createElement('div');
    document.body.appendChild(host);
    app = mount(TextField, { target: host, props }) as unknown as Record<string, never>;
    return host;
  }

  it('renders label plus input with value', () => {
    const el = render({
      dark: 'dark-theme',
      cls: 'obs-setting',
      label: 'Host',
      labelFor: 'obs-HOST',
      id: 'obs-HOST',
      name: 'settings.obs.host',
      value: 'localhost',
    });
    expect(el.querySelector('label')?.getAttribute('for')).toBe('obs-HOST');
    const input = el.querySelector('#obs-HOST') as HTMLInputElement;
    expect(input.getAttribute('name')).toBe('settings.obs.host');
    expect(input.value).toBe('localhost');
  });

  it('wraps password fields with a visibility toggle', () => {
    const el = render({
      dark: '',
      cls: 'spotify-setting',
      id: 'spotify-client_secret',
      name: 'settings.spotify_api.client_secret',
      value: '',
      password: true,
      toggleId: 'show-password-spotify',
    });
    const input = el.querySelector('#spotify-client_secret') as HTMLInputElement;
    expect(input.type).toBe('password');
    expect(el.querySelector('.password-container')).not.toBeNull();
    expect(el.querySelector('#show-password-spotify')).not.toBeNull();
    expect(input.hasAttribute('value')).toBe(false);
  });

  it('escapes values', () => {
    const el = render({ dark: '', cls: 'x', id: 'x', name: 'x', value: '"><script>alert(1)</script>' });
    expect(el.innerHTML).not.toContain('<script>');
    expect(
      (el.querySelector('#x') as HTMLInputElement).value
    ).toBe('"><script>alert(1)</script>');
  });

  it('toggles password visibility through the global', () => {
    const toggle = vi.fn();
    vi.stubGlobal('togglePasswordVisibility', undefined);
    window.togglePasswordVisibility = toggle;
    const el = render({
      dark: '',
      cls: 'spotify-setting',
      id: 'spotify-client_secret',
      name: 'settings.spotify_api.client_secret',
      value: '',
      password: true,
      toggleId: 'show-password-spotify',
    });
    (el.querySelector('#show-password-spotify') as HTMLElement).click();
    expect(toggle).toHaveBeenCalledWith('spotify-client_secret', 'show-password-spotify');
  });
});
