import type {
  Command,
  CommandEvent,
  ConfigResponse,
  DeckBoot,
  DeviceApproval,
  DeviceList,
  DeviceRequest,
  FileSource,
  NativeSelection,
} from './contracts';
import { io, type Socket } from 'socket.io-client';
import schema from '../../contracts/v2.schema.json';
type Schema = {
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
export class ApiError extends Error {
  constructor(
    public status: number,
    message: string,
  ) {
    super(message);
  }
}
let token = sessionStorage.getItem('webdeck.device') ?? '';
export function setToken(t: string) {
  token = t.trim();
  if (token) sessionStorage.setItem('webdeck.device', token);
  else sessionStorage.removeItem('webdeck.device');
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
      throw new Error('Request timed out; execution outcome is unknown. No retry was sent.');
    throw e;
  }
  const v: unknown = await r.json();
  if (!r.ok) {
    const e = v as { message?: string };
    throw new ApiError(r.status, e.message ?? 'Request failed');
  }
  return contract<T>(name, v);
}
export const boot = () => request<DeckBoot>('boot', 'DeckBoot');
export const config = () => request<ConfigResponse>('config', 'ConfigResponse');
export const save = (revision: number, config: ConfigResponse['config']) =>
  request<ConfigResponse>('config', 'ConfigResponse', {
    method: 'PUT',
    body: JSON.stringify({ revision, config }),
  });
export const devices = () => request<DeviceList>('devices', 'DeviceList');
export const approve = (r: DeviceRequest) =>
  request<DeviceApproval>('devices', 'DeviceApproval', { method: 'POST', body: JSON.stringify(r) });
export async function revoke(id: string) {
  await request(`devices/${encodeURIComponent(id)}`, 'DeviceRevocation', { method: 'DELETE' });
}
export async function upload(file: File) {
  const body = new FormData();
  body.set('file', file);
  return request<FileSource>('assets', 'FileSource', { method: 'POST', body });
}
export async function select(kind: NativeSelection['kind']) {
  return request<{ api_version: 2; source: FileSource | null }>(
    'native/selection',
    'SelectionResponse',
    { method: 'POST', body: JSON.stringify({ kind }) },
  );
}
export async function asset(id: string) {
  const headers = new Headers();
  if (token) headers.set('Authorization', `Bearer ${token}`);
  const r = await fetch(`/api/v2/assets/${encodeURIComponent(id)}`, { headers });
  if (!r.ok) throw new Error('Asset unavailable');
  return URL.createObjectURL(await r.blob());
}
export function id() {
  const b = new Uint8Array(16);
  crypto.getRandomValues(b);
  return Array.from(b, (b) => b.toString(16).padStart(2, '0')).join('');
}
let socket: Socket | undefined;
type Pending = {
  resolve: (event: CommandEvent) => void;
  reject: (e: Error) => void;
  timer: ReturnType<typeof setTimeout>;
};
const pending = new Map<string, Pending>();
function disconnect() {
  socket?.disconnect();
  socket = undefined;
  for (const p of pending.values()) {
    clearTimeout(p.timer);
    p.reject(new Error('Connection lost; execution outcome is unknown. No retry was sent.'));
  }
  pending.clear();
}
export function connect() {
  if (socket) return;
  socket = io('/v2', { auth: token ? { token } : {}, autoConnect: false });
  socket.on('command_result', (v: unknown) => {
    let event: CommandEvent;
    try {
      event = contract('CommandEvent', v);
    } catch {
      return;
    }
    const p = pending.get(event.request_id);
    if (!p || event.state === 'accepted') return;
    clearTimeout(p.timer);
    pending.delete(event.request_id);
    p.resolve(event);
  });
  socket.on('disconnect', () => {
    for (const p of pending.values()) {
      clearTimeout(p.timer);
      p.reject(new Error('Connection lost; execution outcome is unknown. No retry was sent.'));
    }
    pending.clear();
  });
  socket.connect();
}
export async function execute(
  command: Command,
  transport: 'http' | 'socket',
): Promise<CommandEvent> {
  const request_id = id();
  const body = { request_id, command };
  contract('CommandRequest', body);
  if (transport === 'http')
    return request('commands', 'CommandEvent', { method: 'POST', body: JSON.stringify(body) });
  connect();
  if (!socket?.connected) throw new Error('Realtime connection is offline');
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => {
      pending.delete(request_id);
      reject(new Error('Result timed out; execution outcome is unknown. No retry was sent.'));
    }, 35000);
    pending.set(request_id, { resolve, reject, timer });
    socket?.volatile.emit('command', body);
  });
}
