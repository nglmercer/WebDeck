import type { ComponentProps } from 'svelte';
import { mount, tick, unmount } from 'svelte';
import { afterEach, describe, expect, it } from 'vitest';
import ColorField from './ColorField.svelte';

describe('ColorField', () => {
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

  function render(props: ComponentProps<typeof ColorField>): HTMLElement {
    host = document.createElement('div');
    document.body.appendChild(host);
    app = mount(ColorField, { target: host, props }) as unknown as Record<string, never>;
    return host;
  }

  it('renders a synced color/hex pair', () => {
    const el = render({
      dark: 'dark-theme',
      containerClass: 'names-color-input-container',
      colorClass: 'names-color-input',
      colorId: 'names-color-input',
      hexClass: 'names-color-setting',
      hexId: 'names-color-hex',
      hexName: 'front.names_color',
      placeholder: 'Button names color (HEX)',
      value: '#ff0000',
    });
    const color = el.querySelector('#names-color-input') as HTMLInputElement;
    expect(color.type).toBe('color');
    expect(color.value).toBe('#ff0000');
    const hex = el.querySelector('#names-color-hex') as HTMLInputElement;
    expect(hex.getAttribute('name')).toBe('front.names_color');
    expect(hex.value).toBe('#ff0000');
  });

  it('omits the value attribute when empty', () => {
    const el = render({
      dark: '',
      containerClass: 'c',
      colorClass: 'cc',
      colorId: 'color',
      hexClass: 'hc',
      hexId: 'hex',
      value: '',
    });
    expect((el.querySelector('#color') as HTMLInputElement).hasAttribute('value')).toBe(false);
    expect((el.querySelector('#hex') as HTMLInputElement).hasAttribute('value')).toBe(false);
  });

  it('syncs both inputs with normalization either way', async () => {
    const el = render({
      dark: '',
      containerClass: 'c',
      colorClass: 'cc',
      colorId: 'color',
      hexClass: 'hc',
      hexId: 'hex',
      value: '#ff0000',
    });
    const color = el.querySelector('#color') as HTMLInputElement;
    const hex = el.querySelector('#hex') as HTMLInputElement;

    hex.value = '00ff00';
    hex.dispatchEvent(new Event('input', { bubbles: true }));
    await tick();
    expect(hex.value).toBe('#00ff00');
    expect(color.value).toBe('#00ff00');

    color.value = '#0000ff';
    color.dispatchEvent(new Event('input', { bubbles: true }));
    await tick();
    expect(color.value).toBe('#0000ff');
    expect(hex.value).toBe('#0000ff');
  });
});
