import { editorPersistence } from '../features/editor/persistence';
import { text } from '../framework/i18n';
import type { BootContext, JsonObject } from '../framework/types';
import { getConfigSnapshot, getJson, postJson } from './client';

/** Shape of every `{success, message?}` JSON answer. */
export interface SaveResult {
  success?: boolean;
  message?: string;
  revision?: number;
}

/** Full boot payload for the SPA. */
export function fetchBoot(): Promise<BootContext> {
  return getJson<BootContext>('/api/boot');
}

/** Current server config (editor entry, add-button save). */
export async function fetchConfig(): Promise<JsonObject> {
  try {
    const snapshot = await getConfigSnapshot<JsonObject>('/get_config');
    editorPersistence.seed(snapshot.config, snapshot.revision);
    return snapshot.config;
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
    return await editorPersistence.save(revision => postJson<SaveResult>('/save_config', data, revision === undefined ? {} : { revision }));
  } catch (error) {
    if (error instanceof Error && error.message.includes('draft is preserved')) throw error;
    throw new Error(text('settings_save_error'));
  }
}
