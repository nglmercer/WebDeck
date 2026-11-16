import { afterEach, expect, it, vi } from 'vitest';
import { mount, unmount, tick } from 'svelte';
import ActionPickerModal from './ActionPickerModal.svelte';
import type { PluginManifest } from '../../lib/contracts';
import type { ActionOption } from './action-options';

const result = { type: 'object', required: true } as const;
const sonora = {
  schema_version: 2,
  id: 'sonora',
  version: '1.0.0',
  entry: 'index.js',
  backend: 'sandbox_js',
  digest: 'x',
  origin: 'local',
  contract: '',
  actions: [
    {
      id: 'current',
      label: 'Sonora: Current track',
      capabilities: ['read'],
      arguments: {},
      result,
    },
    {
      id: 'volume',
      label: 'Sonora: Set volume',
      capabilities: ['audio'],
      arguments: { percent: { type: 'number', required: true } },
      result,
    },
  ],
} as unknown as PluginManifest;

const mounted: Array<ReturnType<typeof mount>> = [];
afterEach(() => {
  for (const component of mounted.splice(0)) void unmount(component);
  document.body.innerHTML = '';
});

async function open(plugins: PluginManifest[], select = vi.fn((_: ActionOption) => {})) {
  const target = document.createElement('div');
  document.body.append(target);
  mounted.push(
    mount(ActionPickerModal, {
      target,
      props: { value: '', close: vi.fn(), select, plugins },
    }),
  );
  await tick();
  return { target, select };
}

async function search(target: HTMLElement, text: string) {
  const input = target.querySelector<HTMLInputElement>('input[type="search"], input[type="text"]');
  if (!input) throw new Error('search field not found');
  input.value = text;
  input.dispatchEvent(new Event('input', { bubbles: true }));
  await tick();
}

it('finds a loaded plugin by its name and selects one of its actions', async () => {
  const { target, select } = await open([sonora]);
  await search(target, 'sonora');
  const volume = target.querySelector<HTMLButtonElement>('[data-action-id="plugin:sonora:volume"]');
  expect(volume).not.toBeNull();
  expect(target.querySelector('[data-action-id="plugin:sonora:current"]')).not.toBeNull();
  volume!.click();
  await tick();
  target.querySelector<HTMLButtonElement>('button.primary')!.click();
  expect(select).toHaveBeenCalledTimes(1);
  expect(select.mock.calls[0]![0]).toMatchObject({
    plugin: { pluginId: 'sonora', actionId: 'volume' },
  });
});

it('shows no plugin entries when no plugin is loaded', async () => {
  const { target } = await open([]);
  await search(target, 'sonora');
  expect(target.querySelector('[data-action-id^="plugin:"]')).toBeNull();
  expect(target.querySelector('[role="status"]')).not.toBeNull();
});
