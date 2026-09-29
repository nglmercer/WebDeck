// Minimal HTML builder: `html` tagged template with automatic escaping,
// `join()` for lists. Mirrors Jinja's autoescape semantics for `{{ }}`
// vs pre-rendered blocks. `Html` is a wrapper class so nesting never
// double-escapes.

export class Html {
  constructor(readonly value: string) {}

  toString(): string {
    return this.value;
  }
}

function escapeHtml(value: string): string {
  return value
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#x27;');
}

function renderValue(value: unknown): string {
  if (value instanceof Html) return value.value;
  if (value === null || value === undefined || value === false) return '';
  if (typeof value === 'string') return escapeHtml(value);
  if (typeof value === 'number' || typeof value === 'boolean') return String(value);
  if (Array.isArray(value)) return value.map(renderValue).join('');
  return escapeHtml(String(value));
}

/**
 * Build escaped HTML. Interpolated `Html` values (from nested `html`
 * calls) pass through untouched; everything else is escaped.
 */
export function html(strings: TemplateStringsArray, ...values: unknown[]): Html {
  let out = '';
  for (let i = 0; i < strings.length; i++) {
    out += strings[i] ?? '';
    if (i < values.length) out += renderValue(values[i]);
  }
  return new Html(out);
}

/** Join pre-rendered fragments without additional escaping. */
export function join(fragments: Html[]): Html {
  return new Html(fragments.map((f) => f.value).join(''));
}
