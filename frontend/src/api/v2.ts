import { getJson, postJson, deleteJson } from './client';
import type { DeviceRequest, DeviceApproval, DeviceList, DeviceRevocation, CommandRequest, CommandEvent } from '../contracts/v2';
export function approveDevice(request: DeviceRequest): Promise<DeviceApproval> { return postJson('/api/v2/devices', request); }
export function listDevices(): Promise<DeviceList> { return getJson('/api/v2/devices'); }
export function revokeDevice(id: string): Promise<DeviceRevocation> { return deleteJson('/api/v2/devices/' + encodeURIComponent(id)); }
export function executeV2(request: CommandRequest): Promise<CommandEvent> { return postJson('/api/v2/commands', request); }
