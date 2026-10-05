import schema from '../../../contracts/v2.schema.json';
type Schema = {
  'x-capability'?: string;
  $ref?: string;
  oneOf?: Schema[];
  const?: unknown;
  enum?: unknown[];
  type?: string;
  properties?: Record<string, Schema>;
  additionalProperties?: boolean | Schema;
  required?: string[];
  items?: Schema;
  minimum?: number;
  maximum?: number;
  minLength?: number;
  maxLength?: number;
  minItems?: number;
  maxItems?: number;
  pattern?: string;
};
const definitions = schema.$defs as Record<string, Schema>;
function valid(s: Schema, v: unknown, depth = 0): boolean {
  if (depth > 64) return false;
  if (s.$ref) return valid(definitions[s.$ref.split('/').pop() ?? ''] ?? {}, v, depth + 1);
  if (s.oneOf) return s.oneOf.filter((s) => valid(s, v, depth + 1)).length === 1;
  if ('const' in s && v !== s.const) return false;
  if (s.enum && !s.enum.includes(v)) return false;
  switch (s.type) {
    case 'object':
      if (!v || typeof v !== 'object' || Array.isArray(v)) return false;
      {
        const o = v as Record<string, unknown>;
        if (s.required?.some((k) => !(k in o))) return false;
        return Object.entries(o).every(([k, v]) =>
          s.properties?.[k]
            ? valid(s.properties[k], v, depth + 1)
            : s.additionalProperties !== false &&
              (typeof s.additionalProperties !== 'object' ||
                valid(s.additionalProperties, v, depth + 1)),
        );
      }
    case 'array':
      return (
        Array.isArray(v) &&
        v.length >= (s.minItems ?? 0) &&
        v.length <= (s.maxItems ?? Infinity) &&
        v.every((v) => valid(s.items ?? {}, v, depth + 1))
      );
    case 'string':
      return (
        typeof v === 'string' &&
        !v.includes('\0') &&
        Array.from(v).length >= (s.minLength ?? 0) &&
        Array.from(v).length <= (s.maxLength ?? Infinity) &&
        (!s.pattern || new RegExp(s.pattern).test(v))
      );
    case 'integer':
      return (
        typeof v === 'number' &&
        Number.isSafeInteger(v) &&
        v >= (s.minimum ?? -Infinity) &&
        v <= (s.maximum ?? Infinity)
      );
    case 'number':
      return (
        typeof v === 'number' &&
        Number.isFinite(v) &&
        v >= (s.minimum ?? -Infinity) &&
        v <= (s.maximum ?? Infinity)
      );
    case 'null':
      return v === null;
    case 'boolean':
      return typeof v === 'boolean';
    default:
      return true;
  }
}
export function contract<T>(name: string, v: unknown): T {
  if (!definitions[name] || !valid(definitions[name], v))
    throw new Error(`Invalid ${name} response`);
  return v as T;
}
export function resolve(s: Schema): Schema {
  return s.$ref ? resolve(definitions[s.$ref.split('/').pop() ?? ''] ?? {}) : s;
}
export const commandSchema = definitions.Command ?? {};
export function defaultValue(raw: Schema): unknown {
  const s = resolve(raw);
  if (s.oneOf) return defaultValue(s.oneOf[0] ?? {});
  if ('const' in s) return s.const;
  if (s.enum) return s.enum[0];
  if (s.type === 'object')
    return Object.fromEntries(
      Object.entries(s.properties ?? {}).map(([k, s]) => [k, defaultValue(s)]),
    );
  if (s.type === 'array')
    return Array.from({ length: s.minItems ?? 0 }, () => defaultValue(s.items ?? {}));
  return s.type === 'integer' || s.type === 'number'
    ? Math.max(0, s.minimum ?? 0)
    : s.type === 'boolean'
      ? false
      : '';
}
export type { Schema };
