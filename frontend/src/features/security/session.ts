const KEY = 'webdeck.device-token';
let token: string | undefined;
export function deviceToken(): string | undefined {
  if (token !== undefined) return token || undefined;
  try { token = sessionStorage.getItem(KEY) ?? ''; } catch { token = ''; }
  return token || undefined;
}
export function setDeviceToken(value: string): void {
  token = value.trim();
  try { if (token) sessionStorage.setItem(KEY, token); else sessionStorage.removeItem(KEY); } catch { /* Memory-only on restricted browsers. */ }
}
