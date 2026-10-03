import { it, expect, vi } from 'vitest';
import { AssetCache } from './assets';
it('deduplicates assets, reuses retained URLs and disposes removed URLs', async () => {
  const fetch = vi.fn(async (id: string) => `blob:${id}`);
  const revoke = vi.fn();
  const cache = new AssetCache(fetch, revoke);
  await cache.load(['a', 'a', 'b']);
  expect(fetch).toHaveBeenCalledTimes(2);
  await cache.load(['a']);
  expect(fetch).toHaveBeenCalledTimes(2);
  expect(revoke).toHaveBeenCalledWith('blob:b');
  cache.dispose();
  expect(revoke).toHaveBeenCalledWith('blob:a');
});
it('revokes a stale response without replacing the current asset set', async () => {
  let finish!: (url: string) => void;
  let started!: () => void;
  const pending = new Promise<void>((resolve) => (started = resolve));
  const revoke = vi.fn();
  const cache = new AssetCache(
    (id) =>
      id === 'old'
        ? new Promise((resolve) => {
            finish = resolve;
            started();
          })
        : Promise.resolve('blob:new'),
    revoke,
  );
  const old = cache.load(['old']);
  await pending;
  expect(await cache.load(['new'])).toEqual({ urls: { new: 'blob:new' }, missing: [] });
  finish('blob:old');
  expect(await old).toBeNull();
  expect(revoke).toHaveBeenCalledWith('blob:old');
  expect(revoke).not.toHaveBeenCalledWith('blob:new');
});

it('bounds all overlapping loads together and skips obsolete queued assets', async () => {
  const pending = new Map<string, (value: string) => void>();
  let active = 0,
    peak = 0,
    started = 0;
  let oldStarted!: () => void, newStarted!: () => void;
  const firstFour = new Promise<void>((resolve) => {
    oldStarted = resolve;
  });
  const nextFour = new Promise<void>((resolve) => {
    newStarted = resolve;
  });
  const fetch = vi.fn(
    (id: string) =>
      new Promise<string>((resolve) => {
        active++;
        peak = Math.max(peak, active);
        pending.set(id, (value) => {
          active--;
          resolve(value);
        });
        started++;
        if (started === 4) oldStarted();
        if (started === 8) newStarted();
      }),
  );
  const revoke = vi.fn();
  const cache = new AssetCache(fetch, revoke);
  const previous = cache.load(['old1', 'old2', 'old3', 'old4', 'obsolete']);
  await firstFour;
  const current = cache.load(['new1', 'new2', 'new3', 'new4']);
  for (const id of ['old1', 'old2', 'old3', 'old4']) pending.get(id)!(`blob:${id}`);
  await nextFour;
  expect(peak).toBe(4);
  expect(fetch).not.toHaveBeenCalledWith('obsolete');
  for (const id of ['new1', 'new2', 'new3', 'new4']) pending.get(id)!(`blob:${id}`);
  expect(await previous).toBeNull();
  expect((await current)?.urls).toEqual({
    new1: 'blob:new1',
    new2: 'blob:new2',
    new3: 'blob:new3',
    new4: 'blob:new4',
  });
  expect(revoke).toHaveBeenCalledWith('blob:old1');
  expect(revoke).not.toHaveBeenCalledWith('blob:new1');
  cache.dispose();
});
