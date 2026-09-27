import { afterEach, describe, expect, it, vi } from 'vitest';
import { get, getJSON, HttpError, post } from './index';

function jsonResponse(payload: unknown, status = 200): Response {
  return new Response(JSON.stringify(payload), {
    status,
    headers: { 'Content-Type': 'application/json' },
  });
}

afterEach(() => {
  vi.unstubAllGlobals();
});

describe('ajax', () => {
  it('getJSON() parses typed JSON', async () => {
    const fetch = vi.fn(async () => jsonResponse({ ok: true, n: 3 }));
    vi.stubGlobal('fetch', fetch);
    const data = await getJSON<{ ok: boolean; n: number }>('/api/status');
    expect(data).toEqual({ ok: true, n: 3 });
    expect(fetch).toHaveBeenCalledWith(
      '/api/status',
      expect.objectContaining({ method: 'GET' })
    );
  });

  it('get() sniffs text responses', async () => {
    vi.stubGlobal('fetch', vi.fn(async () => new Response('hello', { status: 200 })));
    await expect(get<string>('/api/text')).resolves.toBe('hello');
  });

  it('post() JSON-encodes plain bodies', async () => {
    const fetch = vi.fn(async () => jsonResponse({ saved: true }));
    vi.stubGlobal('fetch', fetch);
    await post('/api/save', { a: 1 });
    expect(fetch).toHaveBeenCalledWith(
      '/api/save',
      expect.objectContaining({
        method: 'POST',
        body: JSON.stringify({ a: 1 }),
        headers: expect.objectContaining({ 'Content-Type': 'application/json' }),
      })
    );
  });

  it('throws HttpError on HTTP failures', async () => {
    vi.stubGlobal('fetch', vi.fn(async () => new Response('nope', { status: 500, statusText: 'boom' })));
    const error = await get('/api/bad').catch((e: unknown) => e);
    expect(error).toBeInstanceOf(HttpError);
    expect((error as HttpError).status).toBe(500);
    expect((error as HttpError).url).toBe('/api/bad');
  });

  it('rejects with HttpError on timeout', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn(async (_url: string, init?: { signal?: AbortSignal }) => {
        await new Promise((_, reject) => {
          init?.signal?.addEventListener('abort', () => reject(new DOMException('aborted', 'AbortError')));
        });
        return new Response('late');
      })
    );
    const error = await get('/api/slow', { timeout: 5 }).catch((e: unknown) => e);
    expect(error).toBeInstanceOf(HttpError);
    expect((error as HttpError).status).toBe(0);
  });

  it('propagates network rejections untouched', async () => {
    const failure = new TypeError('offline');
    vi.stubGlobal('fetch', vi.fn(async () => {
      throw failure;
    }));
    await expect(get('/api/down')).rejects.toBe(failure);
  });
});
