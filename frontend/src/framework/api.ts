// Typed wrappers around the existing JSON routes. No new API shapes.

export type Json = Record<string, unknown>;

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(path, init);
  const data: unknown = await response.json().catch(() => null);
  if (!response.ok) {
    const message =
      typeof data === 'object' && data !== null && 'message' in data
        ? String((data as Json)['message'])
        : `HTTP ${response.status}`;
    throw new Error(message);
  }
  return data as T;
}

export function getJson<T>(path: string): Promise<T> {
  return request<T>(path, { method: 'GET' });
}

export function postJson<T>(path: string, body: unknown): Promise<T> {
  return request<T>(path, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(body),
  });
}

export function postText(path: string, query = ''): Promise<string> {
  return fetch(path + query, { method: 'POST' }).then((r) => r.text());
}
