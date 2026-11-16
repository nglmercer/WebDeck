import { afterEach, expect, it, vi } from 'vitest';
import { mount, unmount, tick } from 'svelte';
import Fields from './Fields.svelte';

const mounted: Array<ReturnType<typeof mount>> = [];
afterEach(() => {
  for (const component of mounted.splice(0)) void unmount(component);
  document.body.innerHTML = '';
});

async function render(schema: Record<string, unknown>, value: unknown, onchange = vi.fn()) {
  const target = document.createElement('div');
  document.body.append(target);
  mounted.push(
    mount(Fields, {
      target,
      props: { schema, value, onchange, label: 'Percent', fieldName: 'percent' },
    }),
  );
  await tick();
  return { target, onchange };
}

it('renders a short bounded number as a slider that reports its value', async () => {
  const { target, onchange } = await render({ type: 'number', minimum: 0, maximum: 100 }, 40);
  const slider = target.querySelector<HTMLInputElement>('input[type="range"]');
  expect(slider).not.toBeNull();
  expect(slider!.value).toBe('40');
  slider!.value = '70';
  slider!.dispatchEvent(new Event('input', { bubbles: true }));
  expect(onchange).toHaveBeenCalledWith(70);
});

it('keeps the exact number field for a wide range', async () => {
  const { target } = await render({ type: 'integer', minimum: 0, maximum: 65535 }, 4455);
  expect(target.querySelector('input[type="range"]')).toBeNull();
  expect(target.querySelector<HTMLInputElement>('input[type="number"]')?.value).toBe('4455');
});
