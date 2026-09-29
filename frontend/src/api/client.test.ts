import { afterEach, describe, expect, it, vi } from 'vitest';
import { getJson, getText, HttpError, postForm, postJson, postText } from './client';

function jsonResponse(payload: unknown, status = 200): Response {
  return new Response(JSON.stringify(payload), {
    status,
    headers: { 'Content-Type': 'application/json' },
  });
}

afterEach(() => {
  vi.unstubAllGlobals();
});

describe('api client', () => {
  it('getJson() parses typed JSON', async () => {
    const fetch = vi.fn(async () => jsonResponse({ ok: true, n: 3 }));
    vi.stubGlobal('fetch', fetch);
    const data = await getJson<{ ok: boolean; n: number }>('/api/status');
    expect(data).toEqual({ ok: true, n: 3 });
    expect(fetch).toHaveBeenCalledWith('/api/status', expect.objectContaining({ method: 'GET' }));
  });

  it('getText() reads text bodies', async () => {
    vi.stubGlobal('fetch', vi.fn(async () => new Response('hello', { status: 200 })));
    await expect(getText('/api/text')).resolves.toBe('hello');
  });

  it('postJson() sends JSON with a content type', async () => {
    const fetch = vi.fn(async () => jsonResponse({ saved: true }));
    vi.stubGlobal('fetch', fetch);
    await postJson('/api/save', { a: 1 });
    expect(fetch).toHaveBeenCalledWith(
      '/api/save',
      expect.objectContaining({
        method: 'POST',
        body: JSON.stringify({ a: 1 }),
        headers: expect.objectContaining({ 'Content-Type': 'application/json' }),
      })
    );
  });

  it('postText() posts JSON and reads text', async () => {
    const fetch = vi.fn(async () => new Response('/tmp/dir', { status: 200 }));
    vi.stubGlobal('fetch', fetch);
    await expect(postText('/upload_folderpath')).resolves.toBe('/tmp/dir');
    expect(fetch).toHaveBeenCalledWith(
      '/upload_folderpath',
      expect.objectContaining({ method: 'POST', body: '{}' })
    );
  });

  it('postForm() sends multipart without a JSON content type', async () => {
    const fetch = vi.fn(async () => jsonResponse({ success: true }));
    vi.stubGlobal('fetch', fetch);
    const form = new FormData();
    form.append('file', new File(['x'], 'a.png'));
    await postForm('/upload_file', form);
    const [, init] = fetch.mock.calls[0] as unknown as [string, RequestInit];
    expect(fetch).toHaveBeenCalledWith('/upload_file', expect.objectContaining({ method: 'POST' }));
    expect(init.body).toBe(form);
    expect(init.headers).toBeUndefined();
  });

  it('throws HttpError on HTTP failures', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn(async () => new Response('nope', { status: 500, statusText: 'boom' }))
    );
    const error = await getJson('/api/bad').catch((e: unknown) => e);
    expect(error).toBeInstanceOf(HttpError);
    expect((error as HttpError).status).toBe(500);
    expect((error as HttpError).url).toBe('/api/bad');
  });

  it('carries the server message on JSON error answers', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn(async () => jsonResponse({ success: false, message: 'disk full' }, 500))
    );
    const error = await postJson('/api/save', {}).catch((e: unknown) => e);
    expect(error).toBeInstanceOf(HttpError);
    expect((error as Error).message).toBe('disk full');
  });

  it('rejects with HttpError on timeout', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn(async (_url: string, init?: { signal?: AbortSignal }) => {
        await new Promise((_, reject) => {
          init?.signal?.addEventListener('abort', () =>
            reject(new DOMException('aborted', 'AbortError'))
          );
        });
        return new Response('late');
      })
    );
    const error = await getJson('/api/slow', { timeout: 5 }).catch((e: unknown) => e);
    expect(error).toBeInstanceOf(HttpError);
    expect((error as HttpError).status).toBe(0);
  });

  it('composes caller abort signals', async () => {
    const controller = new AbortController();
    let seen: AbortSignal | undefined;
    vi.stubGlobal(
      'fetch',
      vi.fn(async (_url: string, init?: { signal?: AbortSignal }) => {
        seen = init?.signal;
        await new Promise((_, reject) => {
          init?.signal?.addEventListener('abort', () =>
            reject(new DOMException('aborted', 'AbortError'))
          );
        });
        return new Response('late');
      })
    );
    const pending = getJson('/api/x', { signal: controller.signal });
    controller.abort();
    await expect(pending).rejects.toThrow(DOMException);
    expect(seen?.aborted).toBe(true);
  });

  it('propagates network rejections untouched', async () => {
    const failure = new TypeError('offline');
    vi.stubGlobal(
      'fetch',
      vi.fn(async () => {
        throw failure;
      })
    );
    await expect(getJson('/api/down')).rejects.toBe(failure);
  });
});
