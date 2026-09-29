import type { JsonObject } from '../framework/types';
import { postJson } from './client';

/** Both `/usage` request shapes (the server currently ignores the body). */
export type UsageRequest =
  | { message: string }
  | { messages: Array<{ form: HTMLFormElement; message: string }> };

/** Poll system-usage values for the usage tiles. */
export function fetchUsage(body: UsageRequest): Promise<JsonObject> {
  return postJson<JsonObject>('/usage', body);
}
