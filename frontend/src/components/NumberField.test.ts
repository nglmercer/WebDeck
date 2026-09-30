import type { ComponentProps } from 'svelte';
import { mount, unmount } from 'svelte';
import { afterEach, describe, expect, it } from 'vitest';
import NumberField from './NumberField.svelte';

describe('NumberField', () => {
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

  function render(props: ComponentProps<typeof NumberField>): HTMLElement {
    host = document.createElement('div');
    document.body.appendChild(host);
    app = mount(NumberField, { target: host, props }) as unknown as Record<string, never>;
    return host;
  }

  it('renders a digit-scrubbed number input with fallback value', () => {
    const el = render({
      dark: 'dark-theme',
      id: 'usage-reload-time',
      name: 'front.computer_usage_reload_time',
      label: 'Reload time',
      value: '',
      defaultValue: '3000',
      required: true,
    });
    const input = el.querySelector('#usage-reload-time') as HTMLInputElement;
    expect(input.type).toBe('number');
    expect(input.required).toBe(true);
    expect(input.value).toBe('3000');
  });

  it('scrubs non-digits on input', () => {
    const el = render({
      dark: '',
      id: 'n',
      name: 'n',
      value: '12',
    });
    const input = el.querySelector('#n') as HTMLInputElement;
    input.value = '1a2b3';
    input.dispatchEvent(new Event('input', { bubbles: true }));
    expect(input.value).toBe('123');
  });
});
