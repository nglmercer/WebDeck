import { afterEach, expect, it, vi } from 'vitest';
import { mount, unmount, tick } from 'svelte';
import IntegrationSettings from './IntegrationSettings.svelte';
import type { CatalogResponse } from '../../lib/contracts';

const mounted: Array<ReturnType<typeof mount>> = [];
afterEach(() => {
  for (const component of mounted.splice(0)) void unmount(component);
  document.body.innerHTML = '';
  vi.unstubAllGlobals();
});

const catalog = {
  api_version: 2,
  commands: [],
  plugins: [
    {
      id: 'sonora',
      version: '1.0.0',
      automation: {
        integrations: [
          {
            id: 'sonora.connection',
            label: 'Sonora',
            required_settings: [],
            configuration: [],
            probe: {
              type: 'plugin',
              plugin_id: 'sonora',
              version: '1.0.0',
              action_id: 'providers',
              args: {},
            },
            success: { pointer: '/value/providers' },
          },
        ],
        button_recipes: [],
        presentations: [],
      },
    },
  ],
} as unknown as CatalogResponse;

const editor = {
  dirty: false,
  change: vi.fn(),
  draft: {
    settings: {
      obs: { host: '127.0.0.1', port: 4455, password: '' },
      spotify: { client_id: '', client_secret: '', redirect_uri: '' },
    },
  },
} as never;

it('shows a card for each integration a loaded plugin declares, with a check button', async () => {
  vi.stubGlobal(
    'fetch',
    vi.fn(
      async () =>
        new Response(
          JSON.stringify({
            api_version: 2,
            obs: 'not_tested',
            spotify: 'not_configured',
            checked_at: 0,
            integrations: {
              obs: { state: 'not_tested', checked_at: 0 },
              spotify: { state: 'not_configured', checked_at: 0 },
              'sonora.connection': { state: 'not_tested', checked_at: 0 },
            },
          }),
        ),
    ),
  );
  const target = document.createElement('div');
  document.body.append(target);
  mounted.push(
    mount(IntegrationSettings, {
      target,
      props: {
        editor,
        catalog,
        persist: async () => {},
        attempt: async (work: () => Promise<void>) => work(),
      },
    }),
  );
  await tick();
  await new Promise((resolve) => setTimeout(resolve, 0));
  await tick();
  const card = target.querySelector('#guide-plugin-sonora\\.connection');
  expect(card).not.toBeNull();
  expect(card?.textContent).toContain('Sonora');
  expect(card?.querySelector('button')).not.toBeNull();
});

it('shows no plugin card when no plugin declares an integration', async () => {
  vi.stubGlobal(
    'fetch',
    vi.fn(
      async () =>
        new Response(
          JSON.stringify({
            api_version: 2,
            obs: 'not_tested',
            spotify: 'not_configured',
            checked_at: 0,
          }),
        ),
    ),
  );
  const target = document.createElement('div');
  document.body.append(target);
  mounted.push(
    mount(IntegrationSettings, {
      target,
      props: {
        editor,
        catalog: { api_version: 2, commands: [], plugins: [] } as unknown as CatalogResponse,
        persist: async () => {},
        attempt: async (work: () => Promise<void>) => work(),
      },
    }),
  );
  await tick();
  expect(target.querySelector('[id^="guide-plugin-"]')).toBeNull();
});
