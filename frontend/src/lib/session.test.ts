import { it, expect, vi } from 'vitest';
import { Session, type SessionApi } from './session.svelte';
import type { DeckBoot, Config } from './contracts';
import { clone } from '../features/editor/editor.svelte';
import initial from '../../../webdeck/config_default.json';
const boot = (revision: number): DeckBoot => ({
  api_version: 2 as const,
  revision,
  layout: clone(initial.layout) as Config['layout'],
  can_edit: false,
  language: 'en_US',
  capabilities: ['read'],
  button_capabilities: {},
});
const api = (): SessionApi => ({
  boot: vi.fn(async () => boot(2)),
  config: vi.fn(async () => ({
    api_version: 2 as const,
    revision: 2,
    config: clone(initial) as Config,
  })),
  translations: vi.fn(async () => ({
    api_version: 2 as const,
    language: 'en_US',
    languages: ['en_US'],
    translations: {},
  })),
  catalog: vi.fn(async () => ({ api_version: 2 as const, commands: [], plugins: [] })),
  asset: vi.fn(async (id) => `blob:${id}`),
  connect: vi.fn(),
  disconnect: vi.fn(),
});
it('stale boot responses cannot replace a newer session', async () => {
  const client = api();
  let finish!: (value: DeckBoot) => void;
  client.boot = vi
    .fn()
    .mockImplementationOnce(() => new Promise((resolve) => (finish = resolve)))
    .mockResolvedValueOnce(boot(2));
  const session = new Session({ error: vi.fn(), notice: vi.fn() }, client);
  const old = session.load();
  expect(await session.load()).toBe(true);
  finish(boot(1));
  expect(await old).toBe(false);
  expect(session.deck?.revision).toBe(2);
  expect(client.connect).toHaveBeenCalledTimes(1);
  session.dispose();
});
it('a display refresh failure does not turn a committed save into a write failure', async () => {
  const client = api(),
    notice = vi.fn();
  const session = new Session({ error: vi.fn(), notice }, client);
  await session.load();
  client.boot = vi.fn().mockRejectedValue(new Error('offline'));
  await expect(session.refreshAfterSave()).resolves.toBeUndefined();
  expect(notice).toHaveBeenCalledWith('Saved. Display refresh failed; reload when ready.');
  expect(session.deck?.revision).toBe(2);
  session.dispose();
});

it('late post-save refreshes cannot replace newer boot data or translations', async () => {
  const client = api();
  const session = new Session({ error: vi.fn(), notice: vi.fn() }, client);
  await session.load();
  let finish!: (value: DeckBoot) => void;
  client.boot = vi
    .fn()
    .mockImplementationOnce(
      () =>
        new Promise((resolve) => {
          finish = resolve;
        }),
    )
    .mockResolvedValueOnce(boot(4));
  client.translations = vi
    .fn()
    .mockResolvedValueOnce({
      api_version: 2,
      languages: ['en_US'],
      translations: { marker: 'old' },
    })
    .mockResolvedValueOnce({
      api_version: 2,
      languages: ['es_ES'],
      translations: { marker: 'new' },
    });
  const previous = session.refreshAfterSave();
  await session.refreshAfterSave();
  finish(boot(3));
  await previous;
  expect(session.deck?.revision).toBe(4);
  expect(session.translations.marker).toBe('new');
  expect(session.languages).toEqual(['es_ES']);
  session.dispose();
});

it('an obsolete post-save refresh failure cannot report a failure after a newer success', async () => {
  const client = api();
  const notice = vi.fn();
  const session = new Session({ error: vi.fn(), notice }, client);
  await session.load();
  let fail!: (error: Error) => void;
  client.boot = vi
    .fn()
    .mockImplementationOnce(
      () =>
        new Promise((_resolve, reject) => {
          fail = reject;
        }),
    )
    .mockResolvedValueOnce(boot(4));
  const previous = session.refreshAfterSave();
  await session.refreshAfterSave();
  fail(new Error('old offline request'));
  await previous;
  expect(notice).not.toHaveBeenCalled();
  expect(session.deck?.revision).toBe(4);
  session.dispose();
});
