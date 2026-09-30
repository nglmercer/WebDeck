import { deviceToken } from '../features/security/session';
import { executeV2 } from './v2';
import { editorPersistence } from '../features/editor/persistence';
import { text } from '../framework/i18n';
import { postJson } from './client';
import type { SaveResult } from './config';

/** Payload for `POST /save_single_button` (1-based folder/button indexes). */
export interface SingleButtonPayload {
  location_Folder: string | undefined;
  location_Id: string | undefined;
  content: unknown;
}

/** Payload for `POST /create_folder`. */
export interface CreateFolderInput {
  name: string;
  parent_folder: string;
}

export interface CreateFolderResult {
  success?: boolean;
  message?: string;
}

/** Answer for `POST /send-data`. */
export interface SendResult {
  success?: boolean;
  message?: string;
}

function saveError(): Error {
  return new Error(text('settings_save_error'));
}

/** Persist the whole button grid (add flow, editor save). */
export async function saveButtonsOnly(config: unknown): Promise<void> {
  let result: SaveResult;
  try {
    result = await editorPersistence.save(revision => postJson<SaveResult>('/save_buttons_only', config, revision === undefined ? {} : { revision }));
  } catch (error) {
    if (error instanceof Error && error.message.includes('draft is preserved')) throw error;
    throw saveError();
  }
  if (!result.success) throw saveError();
}

/** Persist one edited button. */
export async function saveSingleButton(payload: SingleButtonPayload): Promise<void> {
  let result: SaveResult;
  try {
    result = await editorPersistence.save(revision => postJson<SaveResult>('/save_single_button', payload, revision === undefined ? {} : { revision }));
  } catch (error) {
    if (error instanceof Error && error.message.includes('draft is preserved')) throw error;
    throw saveError();
  }
  if (!result.success) throw saveError();
}

/** Queue a folder for creation (result says whether it already existed). */
export function createFolder(input: CreateFolderInput): Promise<CreateFolderResult> {
  return postJson<CreateFolderResult>('/create_folder', input);
}

/** Run a button command on the server. */
export async function sendCommand(message: string): Promise<SendResult> {
  if (deviceToken()) {
    const result = await executeV2({ message });
    if (result.state !== 'completed') throw new Error(result.state === 'failed' ? result.message : 'Command is pending');
    return result.result as SendResult;
  }
  return postJson<SendResult>('/send-data', { message });
}
