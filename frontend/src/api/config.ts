import { text } from '../framework/i18n';
import type { BootContext, JsonObject } from '../framework/types';
import { getJson, postJson } from './client';

/** Shape of every `{success, message?}` JSON answer. */
export interface SaveResult {
  success?: boolean;
  message?: string;
}

/** Full boot payload for the SPA. */
export function fetchBoot(): Promise<BootContext> {
  return getJson<BootContext>('/api/boot');
}

/** Current server config (editor entry, add-button save). */
export async function fetchConfig(): Promise<JsonObject> {
  try {
    return await getJson<JsonObject>('/get_config');
  } catch {
    throw new Error(text('settings_load_error'));
  }
}

/**
 * Persist the settings form. Transport failures map to the localized
 * save error; callers branch on the returned result (a `success: false`
 * answer may carry a server message to display).
 */
export async function saveConfig(data: unknown): Promise<SaveResult> {
  try {
    return await postJson<SaveResult>('/save_config', data);
  } catch {
    throw new Error(text('settings_save_error'));
  }
}
