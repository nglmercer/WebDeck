// Loose JSON types mirroring the template context. Templates access the
// config/commands dicts dynamically, so accessors (not rigid interfaces)
// keep the port mechanical and 1:1.

export type JsonValue = string | number | boolean | null | JsonObject | JsonValue[];
export interface JsonObject {
  [key: string]: JsonValue;
}
export type JsonArray = JsonValue[];

export function isObject(value: JsonValue | undefined): value is JsonObject {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

export function asObject(value: JsonValue | undefined): JsonObject {
  return isObject(value) ? value : {};
}

export function asArray(value: JsonValue | undefined): JsonArray {
  return Array.isArray(value) ? value : [];
}

export function asString(value: JsonValue | undefined, fallback = ''): string {
  if (typeof value === 'string') return value;
  if (typeof value === 'number' || typeof value === 'boolean') return String(value);
  return fallback;
}

export function asBool(value: JsonValue | undefined): boolean {
  return value === true || value === 'true' || value === 1;
}

/** Python `str.replace` semantics: replaces ALL occurrences. */
export function rep(value: string, from: string, to: string): string {
  if (from === '') return value;
  return value.split(from).join(to);
}

/** Python `str.replace(old, new, count)`: first `count` occurrences. */
export function repCount(value: string, from: string, to: string, count: number): string {
  if (from === '' || count <= 0) return value;
  let out = '';
  let rest = value;
  for (let i = 0; i < count; i++) {
    const idx = rest.indexOf(from);
    if (idx === -1) break;
    out += rest.slice(0, idx) + to;
    rest = rest.slice(idx + from.length);
  }
  return out + rest;
}

/** Python `str.split()` (no sep): split whitespace runs, strip ends. */
export function pySplit(value: string): string[] {
  const trimmed = value.trim();
  return trimmed === '' ? [] : trimmed.split(/\s+/);
}

/** Safe deep access: `get(config, 'front', 'buttons')` (numeric keys index arrays). */
export function get(obj: JsonValue | undefined, ...keys: string[]): JsonValue | undefined {
  let node = obj;
  for (const key of keys) {
    if (Array.isArray(node)) {
      const index = Number(key);
      if (!Number.isInteger(index)) return undefined;
      node = node[index];
    } else if (isObject(node)) {
      node = node[key];
    } else {
      return undefined;
    }
  }
  return node;
}

export interface BootContext {
  can_edit?: boolean;
  config_revision?: number;
  config: JsonObject;
  commands: JsonObject;
  versions: JsonObject;
  random_bg: string;
  usage_example: JsonObject;
  langs: JsonObject[];
  svgs: string[];
  themes: string[];
  parsed_themes: JsonObject;
  is_exe: boolean;
  portrait_rotate: JsonValue;
  lang: Record<string, string>;
  audio_devices: JsonObject;
  dark_theme: string;
}
