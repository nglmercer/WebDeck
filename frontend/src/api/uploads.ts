import { postForm, postText } from './client';

/** Upload a file to user_uploads (resolves on HTTP 200, like before). */
const pending = new Set<AbortController>();
export async function uploadFile(form: FormData): Promise<void> {
  const controller = new AbortController(); pending.add(controller);
  try { await postForm('/upload_file', form, {signal:controller.signal}); }
  finally { pending.delete(controller); }
}
/** App owns upload lifetimes; a replaced view cannot update the new draft. */
export function cancelUploads(): void { for (const controller of pending) controller.abort(); pending.clear(); }

/** Native folder picker ("" when cancelled). */
export function pickFolderPath(): Promise<string> {
  return postText('/upload_folderpath');
}

/** Native file picker ("" when cancelled). */
export function pickFilePath(filetypes: string | null): Promise<string> {
  let query = '';
  if (filetypes !== null && filetypes.length > 0) {
    query = `?filetypes=${filetypes}`;
  }
  return postText(`/upload_filepath${query}`);
}
