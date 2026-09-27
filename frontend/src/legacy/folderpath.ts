// Port of static/js/folderpath.js. Runs after render.

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
      const folderpathText = document.querySelectorAll('input.folderpath');
      if (folderPath !== '') {
        folderpathText.forEach((textElement) => {
          (textElement as HTMLInputElement).value = folderPath;
        });
      }
    })
    .catch((error) => {
      console.error('Error during request:', error);
    });
}

export function initFolderpath(): void {
  const folderpathButtons = document.querySelectorAll('button.folderpath');

  folderpathButtons.forEach((button) => {
    button.addEventListener('click', () => {
      handleFolderpathButtonClick();
    });
  });

  console.log('folderpath.js loaded');
}
