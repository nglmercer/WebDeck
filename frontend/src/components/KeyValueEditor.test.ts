import { mount, tick, unmount } from 'svelte';
import { afterEach, describe, expect, it } from 'vitest';
import KeyValueEditor from './KeyValueEditor.svelte';

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

async function render(initial: Array<{ name: string; value: string }> = []): Promise<HTMLElement> {
  host = document.createElement('div');
  document.body.appendChild(host);
  app = mount(KeyValueEditor, { target: host, props: { initial } }) as unknown as Record<
    string,
    never
  >;
  await tick();
  return host;
}

function carrier(root: HTMLElement): HTMLInputElement {
  return root.querySelector('input[type="hidden"]') as HTMLInputElement;
}

function rowInputs(root: HTMLElement): NodeListOf<HTMLInputElement> {
  return root.querySelectorAll('input[data-nocollect]');
}

describe('KeyValueEditor', () => {
  it('renders one blank row and an empty carrier by default', async () => {
    const root = await render();
    expect(rowInputs(root).length).toBe(2);
    expect(carrier(root).value).toBe('');
  });

  it('renders initial rows plus a trailing blank row', async () => {
    const root = await render([{ name: 'X-Token', value: 'abc' }]);
    await tick();
    expect(rowInputs(root).length).toBe(4);
    expect(carrier(root).value).toBe('X-Token: abc');
  });

  it('typing in the blank row appends a new blank row', async () => {
    const root = await render();
    const inputs = rowInputs(root);
    (inputs[0] as HTMLInputElement).value = 'A';
    inputs[0]?.dispatchEvent(new Event('input', { bubbles: true }));
    await tick();
    expect(rowInputs(root).length).toBe(4);
    expect(carrier(root).value).toBe('A');
  });

  it('removes rows and updates the carrier', async () => {
    const root = await render([
      { name: 'A', value: '1' },
      { name: 'B', value: '2' },
    ]);
    await tick();
    expect(carrier(root).value).toBe('A: 1\nB: 2');
    const remove = root.querySelector('button.kv-remove') as HTMLButtonElement;
    remove.click();
    await tick();
    expect(carrier(root).value).toBe('B: 2');
  });

  it('skips blank-name rows in the carrier', async () => {
    const root = await render([{ name: '', value: 'orphan' }]);
    await tick();
    expect(carrier(root).value).toBe('');
  });
});
