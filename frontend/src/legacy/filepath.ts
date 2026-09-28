// Port of static/js/filepath.js. Runs after render.
import { q } from '../query';

function handleFilepathButtonClick(filetypes: string | null): void {
  let filetypesString = '';
  if (filetypes != null && filetypes.length > 0) {
    filetypesString = `?filetypes=${filetypes}`;
  }
  fetch(`/upload_filepath${filetypesString}`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify({}),
  })
    .then((response) => response.text())
    .then((filePath) => {
      console.log('File path:', filePath);
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
