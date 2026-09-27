// Typed fetch-based ajax (jQuery.ajax/get/post/getJSON without XHR).

/** Error for HTTP failures (and our own timeouts). Network-level fetch
 * rejections propagate untouched. */
export class HttpError extends Error {
  readonly status: number;
  readonly statusText: string;
  readonly url: string;

  constructor(status: number, statusText: string, url: string) {
    super(`${status} ${statusText} (${url})`);
    this.name = 'HttpError';
    this.status = status;
    this.statusText = statusText;
    this.url = url;
  }
}

export interface AjaxOptions {
  method?: string;
  headers?: Record<string, string>;
  /** Plain objects/arrays are JSON-encoded automatically. */
  body?: unknown;
  /** Milliseconds before aborting (rejects with HttpError, status 0). */
  timeout?: number;
  /** Caller abort signal (composes with `timeout`). */
  signal?: AbortSignal;
  /** Force a response kind; default sniffs the content type. */
  responseType?: 'json' | 'text';
  credentials?: RequestCredentials;
}

function encodeBody(body: unknown): { payload: BodyInit | undefined; json: boolean } {
  if (
    body === undefined ||
    typeof body === 'string' ||
    body instanceof FormData ||
    body instanceof URLSearchParams ||
    body instanceof Blob ||
    body instanceof ArrayBuffer
  ) {
    return { payload: body as BodyInit | undefined, json: false };
  }
  return { payload: JSON.stringify(body), json: true };
}

/**
 * Typed request. `T` defaults to `unknown` — name it:
 * `ajax<{ ok: boolean }>('/api/status')`.
 */
export async function ajax<T = unknown>(url: string, options: AjaxOptions = {}): Promise<T> {
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
  try {
    options.signal?.addEventListener('abort', onCallerAbort, { once: true });
    const { payload, json } = encodeBody(options.body);
    const headers: Record<string, string> = { ...(options.headers ?? {}) };
    if (json && !Object.keys(headers).some((h) => h.toLowerCase() === 'content-type')) {
      headers['Content-Type'] = 'application/json';
    }
    let response: Response;
    try {
      const init: RequestInit = {
        method: options.method ?? (options.body === undefined ? 'GET' : 'POST'),
        headers,
        signal: controller.signal,
      };
      if (payload !== undefined) init.body = payload;
      if (options.credentials !== undefined) init.credentials = options.credentials;
      response = await fetch(url, init);
    } catch (error) {
      if (timedOut) {
        throw new HttpError(0, 'timeout', url);
      }
      throw error;
    }
    if (!response.ok) {
      throw new HttpError(response.status, response.statusText, url);
    }
    const kind =
      options.responseType ??
      (response.headers.get('content-type')?.includes('json') ? 'json' : 'text');
    if (kind === 'json') return (await response.json()) as T;
    return (await response.text()) as unknown as T;
  } finally {
    if (timer !== undefined) clearTimeout(timer);
    options.signal?.removeEventListener('abort', onCallerAbort);
  }
}

/** GET with content-type sniffing (JSON parsed, else text). */
export function get<T = unknown>(url: string, options: AjaxOptions = {}): Promise<T> {
  return ajax<T>(url, { ...options, method: 'GET' });
}

/** GET that must be JSON (rejects on invalid JSON). */
export function getJSON<T = unknown>(url: string, options: AjaxOptions = {}): Promise<T> {
  return ajax<T>(url, { ...options, method: 'GET', responseType: 'json' });
}

/** POST with automatic JSON encoding for plain bodies. */
export function post<T = unknown>(
  url: string,
  body?: unknown,
  options: AjaxOptions = {}
): Promise<T> {
  return ajax<T>(url, { ...options, method: 'POST', body });
}
