import { postForm, postText } from './client';

/** Upload a file to user_uploads (resolves on HTTP 200, like before). */
export function uploadFile(form: FormData): Promise<void> {
  return postForm('/upload_file', form);
}

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
