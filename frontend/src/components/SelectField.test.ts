import type { ComponentProps } from 'svelte';
import { mount, tick, unmount } from 'svelte';
import { afterEach, describe, expect, it } from 'vitest';
import SelectField from './SelectField.svelte';

describe('SelectField', () => {
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

  async function render(props: ComponentProps<typeof SelectField>): Promise<HTMLElement> {
    host = document.createElement('div');
    document.body.appendChild(host);
    app = mount(SelectField, { target: host, props }) as unknown as Record<string, never>;
    await tick();
    return host;
  }

  it('renders options with a single selected entry', async () => {
    const el = await render({
      containerClass: 'language',
      id: 'language',
      name: 'settings.language',
      label: 'Language',
      options: [
        { value: 'en_US', label: 'English (en_US)', selected: false },
        { value: 'es_ES', label: 'Español (es_ES)', selected: true },
      ],
    });
    const select = el.querySelector('#language') as HTMLSelectElement;
    expect(select.getAttribute('name')).toBe('settings.language');
    expect(select.querySelectorAll('option[selected]')).toHaveLength(1);
    expect(select.value).toBe('es_ES');
  });
});
