// Port of static/js/upload_file.js. Runs after render.
import { uploadFile } from '../api/uploads';
import { q } from '../query';

export function upload_file(element: HTMLInputElement): void {
  const file = element.files?.[0];
  if (!file) return;
  const formData = new FormData();
  formData.append('file', file);

  void uploadFile(formData).then(undefined, () => {
    console.error('Failed to download file.');
  });
}

export function initUploadFile(): void {
  q('.audio-input')
    .toArray()
    .forEach(function (element) {
      q(element).on('change', function () {
        upload_file(element as HTMLInputElement);
      });
    });
}
