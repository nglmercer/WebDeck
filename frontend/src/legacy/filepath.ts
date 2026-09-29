// Port of static/js/filepath.js. Runs after render.
import { pickFilePath } from '../api/uploads';
import { q } from '../query';

function handleFilepathButtonClick(filetypes: string | null): void {
  pickFilePath(filetypes)
    .then((filePath) => {
      if (filePath !== '') {
        q('input.filepath').val(filePath);
      }
    })
    .catch((error) => {
      console.error('Error during request:', error);
    });
}

export function initFilepath(): void {
  q('button.filepath')
    .toArray()
    .forEach((button) => {
      const filetypes = q(button).attr('filetypes') ?? null;
      q(button).on('click', () => {
        handleFilepathButtonClick(filetypes);
      });
    });
}
