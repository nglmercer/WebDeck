// Toast helpers (index.jinja showToast/showError/showInfo) + wake lock.
import Toastify from 'toastify-js';
import 'toastify-js/src/toastify.css';
import { q } from '../query';

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
  console.log('ERROR: ', message);
}

export function showInfo(message: string): void {
  showToast(1, message);
  console.log('INFO: ', message);
}

export function wireWakeLock(): void {
  const wakeLock = async (): Promise<void> => {
    try {
      await navigator.wakeLock.request('screen');
    } catch {
      // ignore
    }
  };
  // Every bubbled click passes through <html>, like the document listener did.
  q('html').on('click', () => {
    void wakeLock();
  });
}
