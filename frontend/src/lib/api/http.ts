import { contract } from '../schema';
import { disconnect } from './realtime';
export class ApiError extends Error {
  constructor(
    public status: number,
    message: string,
  ) {
    super(message);
  }
}
let token =
  sessionStorage.getItem('webdeck.device') ?? localStorage.getItem('webdeck.device') ?? '';
export function setToken(t: string, remember = false) {
  token = t.trim();
  if (token) sessionStorage.setItem('webdeck.device', token);
  else sessionStorage.removeItem('webdeck.device');
  if (token && remember) localStorage.setItem('webdeck.device', token);
  else localStorage.removeItem('webdeck.device');
  disconnect();
}
export async function request<T>(path: string, name: string, init: RequestInit = {}): Promise<T> {
  const headers = new Headers(init.headers);
  if (token) headers.set('Authorization', `Bearer ${token}`);
  if (init.body && !(init.body instanceof FormData))
    headers.set('Content-Type', 'application/json');
  let r: Response;
  try {
    r = await fetch(`/api/v2/${path}`, {
      ...init,
      headers,
      signal: init.signal ?? AbortSignal.timeout(35000),
    });
  } catch (e) {
    if (e instanceof DOMException && (e.name === 'TimeoutError' || e.name === 'AbortError'))
      throw new Error(
        path === 'commands'
          ? 'Request timed out; execution outcome is unknown. No retry was sent.'
          : 'Request timed out. Try again.',
      );
    throw e;
  }
  const v: unknown = await r.json();
  if (!r.ok) {
    const e = v as { message?: string };
    throw new ApiError(r.status, e.message ?? 'Request failed');
  }
  return contract<T>(name, v);
}

export function credentials(): Headers {
  const headers = new Headers();
  if (token) headers.set('Authorization', `Bearer ${token}`);
  return headers;
}
export function credential() {
  return token;
}
