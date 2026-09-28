import { mount, unmount } from 'svelte';
import { afterEach, describe, expect, it } from 'vitest';
import SwitchField from './SwitchField.svelte';

describe('SwitchField', () => {
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

  function render(props: Record<string, unknown>): HTMLElement {
    host = document.createElement('div');
    document.body.appendChild(host);
    app = mount(SwitchField, { target: host, props }) as unknown as Record<string, never>;
    return host;
  }

  it('renders the switch row with name and checked state', () => {
    const el = render({
      dark: 'dark-theme',
      containerClass: 'toggle-soundboard',
      label: 'Soundboard enabled',
      id: 'toggle_soundboard',
      name: 'settings.soundboard.enabled',
      checked: true,
    });
    expect(el.querySelector('.setting.toggle-soundboard')).not.toBeNull();
    const input = el.querySelector('#toggle_soundboard') as HTMLInputElement;
    expect(input.getAttribute('name')).toBe('settings.soundboard.enabled');
    // Svelte sets the checked property (what `:checked` CSS and form
    // serialization read), not the content attribute; nothing reads the
    // attribute back (verified: no `[checked]` selectors or attr readers).
    expect(input.checked).toBe(true);
    expect(el.querySelector('span.slider.round')).not.toBeNull();
  });

  it('omits checked and appends extra classes when closed', () => {
    const el = render({
      dark: '',
      containerClass: 'auto-updates',
      label: 'Auto updates',
      id: 'auto-updates',
      name: 'settings.auto_updates',
      checked: false,
      extraClass: 'invisible',
    });
    expect(el.querySelector('.setting.auto-updates.invisible')).not.toBeNull();
    const input = el.querySelector('#auto-updates') as HTMLInputElement;
    expect(input.checked).toBe(false);
    expect(input.hasAttribute('checked')).toBe(false);
  });
});
