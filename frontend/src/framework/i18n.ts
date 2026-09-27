// Client-side `text()` — same fallback rule as the server port:
// missing key renders the key itself.

let dict: Record<string, string> = {};

export function initI18n(entries: Record<string, string>): void {
  dict = entries;
}

export function text(key: string): string {
  return dict[key] ?? key;
}
