// Generated from contracts/v2.schema.json; run node tools/contracts/generate.mjs.
export type Capability = "read" | "input" | "audio" | "window" | "power" | "script" | "network" | "plugin" | "admin" | "settings";
export type ErrorCode = "invalid_input" | "unsupported_schema" | "conflict" | "unauthorized" | "forbidden" | "capacity_exhausted" | "shutting_down" | "unknown_command" | "execution_failed" | "persistence_failed";
export interface CommandRequest {
  message: string;
  request_id?: string;
}
export interface ConfigRequest {
  revision: number;
  config: Record<string, unknown>;
}
export interface DeviceRequest {
  name: string;
  capabilities: Array<Capability>;
  ttl_seconds: number;
}
export interface CommandAccepted {
  api_version: 2;
  request_id: string;
  state: "accepted";
}
export interface CommandCompleted {
  api_version: 2;
  request_id: string;
  state: "completed";
  result: Record<string, unknown>;
}
export interface CommandFailed {
  api_version: 2;
  request_id: string;
  state: "failed";
  code: ErrorCode;
  message: string;
}
export interface ConfigResponse {
  api_version: 2;
  revision: number;
  config: Record<string, unknown>;
}
export interface Device {
  id: string;
  name: string;
  capabilities: Array<Capability>;
  expires_at: number;
  revoked: boolean;
}
export interface DeviceApproval {
  api_version: 2;
  device: Device;
  token: string;
}
export interface DeviceList {
  api_version: 2;
  devices: Array<Device>;
}
export interface DeviceRevocation {
  api_version: 2;
  revoked: true;
}
export type CommandEvent = CommandAccepted | CommandCompleted | CommandFailed;
