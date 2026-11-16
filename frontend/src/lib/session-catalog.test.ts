import { describe, expect, it, vi } from 'vitest';
import { Session, type SessionApi } from './session.svelte';
import type { CatalogResponse } from './contracts';

const catalog = (id: string) =>
  ({
    api_version: 2,
    commands: [{ id, capability: 'read', schema: {} }],
    plugins: [],
  }) as unknown as CatalogResponse;

function session(api: Partial<SessionApi>) {
  const feedback = { error: vi.fn(), notice: vi.fn() };
  return new Session(feedback, api as SessionApi, () => true);
}

describe('catalogue refresh after a plugin reload', () => {
  it('replaces the catalogue with the one asked for again', async () => {
    const next = vi.fn(async () => catalog('fresh'));
    const current = session({ catalog: next });
    current.catalog = catalog('old');
    await current.refreshCatalog();
    expect(next).toHaveBeenCalledTimes(1);
    expect(current.catalog?.commands[0]?.id).toBe('fresh');
  });

  it('keeps the previous catalogue when the request fails', async () => {
    const current = session({
      catalog: vi.fn(async () => Promise.reject(new Error('down'))),
    });
    current.catalog = catalog('old');
    await current.refreshCatalog();
    expect(current.catalog?.commands[0]?.id).toBe('old');
  });
});
