// Port of static/js/filepath.js. Runs after render.

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
      const filepathText = document.querySelectorAll('input.filepath');
      if (filePath !== '') {
        filepathText.forEach((textElement) => {
          (textElement as HTMLInputElement).value = filePath;
        });
      }
    })
    .catch((error) => {
      console.error('Error during request:', error);
    });
}

export function initFilepath(): void {
  const filepathButtons = document.querySelectorAll('button.filepath');

  filepathButtons.forEach((button) => {
    const filetypes = button.getAttribute('filetypes');
    button.addEventListener('click', () => {
      handleFilepathButtonClick(filetypes);
    });
  });

  console.log('filepath.js loaded');
}
