// Port of static/js/folderpath.js. Runs after render.
import { pickFolderPath } from '../api/uploads';
import { q } from '../query';

function handleFolderpathButtonClick(): void {
  pickFolderPath()
    .then((folderPath) => {
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
