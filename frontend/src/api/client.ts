import { deviceToken } from '../features/security/session';
// Single HTTP transport for the app (native fetch).
//
// Centralizes status checks, JSON parsing, server-message extraction,
// and timeout/abort support. Replaces the former framework/api and
// query/ajax implementations.

/**
 * Error for HTTP failures (and our own timeouts). Network-level fetch
 * rejections propagate untouched. `message` carries the server's
 * `{message}` body when the error response had one.
 */
export class HttpError extends Error {
  readonly status: number;
  readonly statusText: string;
  readonly url: string;

  constructor(status: number, statusText: string, url: string, message?: string) {
    super(message ?? `${status} ${statusText} (${url})`);
    this.name = 'HttpError';
    this.status = status;
    this.statusText = statusText;
    this.url = url;
  }
}

export interface RequestOptions {
  /** Milliseconds before aborting (rejects with HttpError, status 0). */
  timeout?: number;
  revision?: number;
  /** Caller abort signal (composes with `timeout`). */
  signal?: AbortSignal;
}

async function send(
  path: string,
  init: RequestInit,
  options: RequestOptions = {}
): Promise<Response> {
  const controller = new AbortController();
  let timedOut = false;
  const timer =
    options.timeout === undefined
      ? undefined
      : setTimeout(() => {
          timedOut = true;
          controller.abort();
        }, options.timeout);
  const onCallerAbort = (): void => controller.abort();
  if (options.signal?.aborted) controller.abort();
  if (options.revision !== undefined) init.headers = { ...init.headers, 'X-WebDeck-Revision': String(options.revision) };
  const token = deviceToken();
  if (token) init.headers = { ...init.headers, Authorization: `Bearer ${token}` };
  try {
    options.signal?.addEventListener('abort', onCallerAbort, { once: true });
    try {
      return await fetch(path, { ...init, signal: controller.signal });
    } catch (error) {
      if (timedOut) {
        throw new HttpError(0, 'timeout', path);
      }
      throw error;
    }
  } finally {
    if (timer !== undefined) clearTimeout(timer);
    options.signal?.removeEventListener('abort', onCallerAbort);
  }
}

/** Throw HttpError for non-OK responses (server `{message}` when present). */
async function checkOk(response: Response, url: string): Promise<void> {
  if (response.ok) return;
  const data: unknown = await response.json().catch(() => null);
  const message =
    typeof data === 'object' && data !== null && 'message' in data
      ? String((data as Record<string, unknown>)['message'])
      : undefined;
  throw new HttpError(response.status, response.statusText, url, message);
}

/** GET with a JSON body. */
export async function getJson<T>(path: string, options: RequestOptions = {}): Promise<T> {
  const response = await send(path, { method: 'GET' }, options);
  await checkOk(response, path);
  return (await response.json()) as T;
}

/** POST a JSON body, parse the JSON answer. */
export async function postJson<T>(
  path: string,
  body: unknown,
  options: RequestOptions = {}
): Promise<T> {
  const response = await send(
    path,
    {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    },
    options
  );
  await checkOk(response, path);
  return (await response.json()) as T;
}

/** GET with a text body. */
export async function getText(path: string, options: RequestOptions = {}): Promise<string> {
  const response = await send(path, { method: 'GET' }, options);
  await checkOk(response, path);
  return response.text();
}

/** POST a JSON body, read the text answer. */
export async function postText(
  path: string,
  body: unknown = {},
  options: RequestOptions = {}
): Promise<string> {
  const response = await send(
    path,
    {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    },
    options
  );
  await checkOk(response, path);
  return response.text();
}

/** POST multipart form data (the response body is ignored). */
export async function postForm(
  path: string,
  form: FormData,
  options: RequestOptions = {}
): Promise<void> {
  const response = await send(path, { method: 'POST', body: form }, options);
  await checkOk(response, path);
}

/** Configuration callers capture the revision with the exact snapshot read. */
export async function getConfigSnapshot<T>(path: string): Promise<{ config: T; revision: number | undefined }> {
  const response = await send(path, { method: 'GET' });
  await checkOk(response, path);
  const raw = response.headers.get('x-webdeck-revision');
  return { config: await response.json() as T, revision: raw === null ? undefined : Number(raw) };
}

export async function deleteJson<T>(path: string): Promise<T> {
  const response = await send(path, {method: 'DELETE'}); await checkOk(response, path); return await response.json() as T;
}
