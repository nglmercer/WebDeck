import type {
  ConfigResponse,
  DeckBoot,
  DeviceApproval,
  DeviceList,
  DeviceRequest,
  FileSource,
  NativeSelection,
  PairingList,
} from '../contracts';
import { request, credentials } from './http';
export { request, setToken, ApiError } from './http';
export { connect, disconnect, execute } from './realtime';
export { contract, resolve, defaultValue, commandSchema, type Schema } from '../schema';
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
export const pairings = () => request<PairingList>('pairing', 'PairingList');
export const approvePairing = (id: string, r: DeviceRequest) =>
  request<PairingList>(`pairing/${encodeURIComponent(id)}/approve`, 'PairingList', {
    method: 'POST',
    body: JSON.stringify(r),
  });
export const rejectPairing = (id: string) =>
  request<PairingList>(`pairing/${encodeURIComponent(id)}`, 'PairingList', { method: 'DELETE' });
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
  const headers = credentials();
  const r = await fetch(`/api/v2/assets/${encodeURIComponent(id)}`, { headers });
  if (!r.ok) throw new Error('Asset unavailable');
  return URL.createObjectURL(await r.blob());
}
