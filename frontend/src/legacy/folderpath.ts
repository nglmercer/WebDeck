// Port of static/js/folderpath.js. Runs after render.
import { q } from '../query';

function handleFolderpathButtonClick(): void {
  fetch('/upload_folderpath', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify({}),
  })
    .then((response) => response.text())
    .then((folderPath) => {
      console.log('Dir path:', folderPath);
      if (folderPath !== '') {
        q('input.folderpath').val(folderPath);
      }
    })
    .catch((error) => {
      console.error('Error during request:', error);
    });
}

export function initFolderpath(): void {
  q('button.folderpath')
    .toArray()
    .forEach((button) => {
      q(button).on('click', () => {
        handleFolderpathButtonClick();
      });
    });
}
