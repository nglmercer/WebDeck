// Port of static/js/upload_file.js. Runs after render.

export function upload_file(element: HTMLInputElement): void {
  const file = element.files?.[0];
  if (!file) return;
  const formData = new FormData();
  formData.append('file', file);

  const xhr = new XMLHttpRequest();
  xhr.open('POST', '/upload_file', true);

  xhr.onload = function () {
    if (xhr.status === 200) {
      console.log('File downloaded successfully!');
    } else {
      console.error('Failed to download file.');
    }
  };

  xhr.send(formData);
}

export function initUploadFile(): void {
  const elements = document.querySelectorAll('.audio-input');
  elements.forEach(function (element) {
    element.addEventListener('change', function () {
      upload_file(element as HTMLInputElement);
    });
  });
}
