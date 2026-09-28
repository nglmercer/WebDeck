import { mount, tick, unmount } from 'svelte';
import { afterEach, describe, expect, it } from 'vitest';
import FileField from './FileField.svelte';

describe('FileField', () => {
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

  async function render(props: Record<string, unknown>): Promise<HTMLElement> {
    host = document.createElement('div');
    document.body.appendChild(host);
    app = mount(FileField, { target: host, props }) as unknown as Record<string, never>;
    await tick();
    return host;
  }

  it('renders one picker row with the empty hint and a hidden native input', async () => {
    const el = await render({
      id: 'image-input_x',
      accept: 'image/*',
      browseLabel: 'Select your file',
      emptyLabel: 'No file chosen',
    });
    const input = el.querySelector('#image-input_x') as HTMLInputElement;
    expect(input.getAttribute('type')).toBe('file');
    expect(input.getAttribute('accept')).toBe('image/*');
    expect(input.className).toContain('wd2-dropfile-input');
    expect(el.querySelector('.wd2-dropfile-action')?.textContent).toBe('Select your file');
    expect(el.querySelector('.wd2-dropfile-name')?.textContent?.trim()).toBe('No file chosen');
    expect(el.querySelector('.wd2-dropfile-name.chosen')).toBeNull();
  });

  it('swaps the hint for the chosen file name on change', async () => {
    const el = await render({
      id: 'create-image-bg',
      accept: 'image/png',
      browseLabel: 'Select your file',
      emptyLabel: 'No file chosen',
      hint: 'PNG',
    });
    const input = el.querySelector('#create-image-bg') as HTMLInputElement;
    const file = new File(['x'], 'pic.png', { type: 'image/png' });
    Object.defineProperty(input, 'files', { value: [file], configurable: true });
    input.dispatchEvent(new Event('change', { bubbles: true }));
    await tick();
    expect(el.querySelector('.wd2-dropfile-name')?.textContent?.trim()).toBe('pic.png');
    expect(el.querySelector('.wd2-dropfile-name.chosen')).not.toBeNull();
  });
});
