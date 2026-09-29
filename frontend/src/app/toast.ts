// Toast helpers (index.jinja showToast/showError).
import Toastify from 'toastify-js';
import 'toastify-js/src/toastify.css';

export function showToast(kind: number, message: string): void {
  let bg: string;
  let duration: number;
  if (kind === 0) {
    bg = 'linear-gradient(to right, #dc4e74, #c93d3d)';
    duration = 2000;
  } else {
    duration = 3000;
    bg = 'linear-gradient(to right, #00b07d, #96c93d)';
  }
  Toastify({
    text: message,
    duration,
    newWindow: true,
    close: true,
    gravity: 'top',
    stopOnFocus: true,
    style: {
      background: bg,
    },
  }).showToast();
}

export function showError(message: string): void {
  showToast(0, message);
  console.error('ERROR: ', message);
}
