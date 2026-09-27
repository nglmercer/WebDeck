import { describe, expect, it } from 'vitest';
import { html } from '../framework/html';
import {
  colorField,
  infoTitle,
  normalizeHexColor,
  numberField,
  selectField,
  switchField,
  textField,
} from './fields';

describe('switchField', () => {
  it('renders the switch row with name and checked state', () => {
    const out = switchField({
      dark: 'dark-theme',
      containerClass: 'toggle-soundboard',
      label: 'Soundboard enabled',
      id: 'toggle_soundboard',
      name: 'settings.soundboard.enabled',
      checked: true,
    }).value;
    expect(out).toContain('class="setting toggle-soundboard"');
    expect(out).toContain('id="toggle_soundboard"');
    expect(out).toContain('name="settings.soundboard.enabled"');
    expect(out).toContain('checked');
    expect(out).toContain('class="slider round"');
  });

  it('omits checked and appends extra classes when closed', () => {
    const out = switchField({
      dark: '',
      containerClass: 'auto-updates',
      label: 'Auto updates',
      id: 'auto-updates',
      name: 'settings.auto_updates',
      checked: false,
      extraClass: 'invisible',
    }).value;
    expect(out).toContain('class="setting auto-updates invisible"');
    expect(out).not.toContain('checked');
  });
});

describe('textField', () => {
  it('renders label plus input with value', () => {
    const out = textField({
      dark: 'dark-theme',
      cls: 'obs-setting',
      label: 'Host',
      labelFor: 'obs-HOST',
      id: 'obs-HOST',
      name: 'settings.obs.host',
      value: 'localhost',
    }).value;
    expect(out).toContain('<label for="obs-HOST">');
    expect(out).toContain('id="obs-HOST"');
    expect(out).toContain('name="settings.obs.host"');
    expect(out).toContain('value="localhost"');
  });

  it('wraps password fields with a visibility toggle', () => {
    const out = textField({
      dark: '',
      cls: 'spotify-setting',
      id: 'spotify-client_secret',
      name: 'settings.spotify_api.client_secret',
      value: '',
      password: true,
      toggleId: 'show-password-spotify',
    }).value;
    expect(out).toContain('type="password"');
    expect(out).toContain('class="password-container"');
    expect(out).toContain('id="show-password-spotify"');
    expect(out).not.toContain('value=');
  });

  it('escapes values', () => {
    const out = textField({
      dark: '',
      cls: 'x',
      id: 'x',
      name: 'x',
      value: '"><script>alert(1)</script>',
    }).value;
    expect(out).not.toContain('<script>');
    expect(out).toContain('&quot;&gt;&lt;script&gt;');
  });
});

describe('selectField', () => {
  it('renders options with a single selected entry', () => {
    const out = selectField({
      containerClass: 'language',
      id: 'language',
      name: 'settings.language',
      label: 'Language',
      options: [
        { value: 'en_US', label: 'English (en_US)', selected: false },
        { value: 'es_ES', label: 'Español (es_ES)', selected: true },
      ],
    }).value;
    expect(out).toContain('<select id="language" name="settings.language">');
    expect(out).toContain('<option value="es_ES" selected>');
    expect(out.match(/selected/g) ?? []).toHaveLength(1);
  });
});

describe('numberField', () => {
  it('renders a digit-scrubbed number input with fallback value', () => {
    const out = numberField({
      dark: 'dark-theme',
      id: 'usage-reload-time',
      name: 'front.computer_usage_reload_time',
      label: 'Reload time',
      value: '',
      defaultValue: '3000',
      required: true,
    }).value;
    expect(out).toContain('type="number"');
    expect(out).toContain('required');
    expect(out).toContain('value="3000"');
    expect(out).toContain('replaceAll(/[^0-9]/g');
  });
});

describe('colorField', () => {
  it('renders a synced color/hex pair', () => {
    const out = colorField({
      dark: 'dark-theme',
      containerClass: 'names-color-input-container',
      colorClass: 'names-color-input',
      colorId: 'names-color-input',
      hexClass: 'names-color-setting',
      hexId: 'names-color-hex',
      hexName: 'front.names_color',
      placeholder: 'Button names color (HEX)',
      value: '#ff0000',
    }).value;
    expect(out).toContain('type="color"');
    expect(out).toContain('id="names-color-input"');
    expect(out).toContain('id="names-color-hex"');
    expect(out).toContain('name="front.names_color"');
    expect(out).toContain('value="#ff0000"');
  });

  it('omits the value attribute when empty', () => {
    const out = colorField({
      dark: '',
      containerClass: 'c',
      colorClass: 'cc',
      colorId: 'color',
      hexClass: 'hc',
      hexId: 'hex',
      value: '',
    }).value;
    expect(out).not.toContain('value=');
  });
});

describe('normalizeHexColor', () => {
  it('prefixes a missing hash and trims', () => {
    expect(normalizeHexColor('  ff0000 ')).toBe('#ff0000');
    expect(normalizeHexColor('#00ff00')).toBe('#00ff00');
    expect(normalizeHexColor('')).toBe('');
    expect(normalizeHexColor('   ')).toBe('');
  });
});

describe('infoTitle', () => {
  it('renders the label plus tutorial link', () => {
    const out = infoTitle({
      label: 'Soundboard',
      labelFor: 'soundboard',
      href: 'https://example.com/docs',
      title: 'Tutorial',
      icon: html`<svg></svg>`,
    }).value;
    expect(out).toContain('class="settings-title-info"');
    expect(out).toContain('for="soundboard"');
    expect(out).toContain('href="https://example.com/docs"');
    expect(out).toContain('<svg></svg>');
  });
});
