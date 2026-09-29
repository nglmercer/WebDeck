import { sendCommand } from '../api/buttons';
import { HttpError } from '../api/client';
import { showError } from './toast';

// Port of send_data() (also exposed as window.send_data for inline onclick).
export function send_data(message: string): void {
  sendCommand(message)
    .then((data) => {
      if (!data.success) {
        console.error(data.message);
        if (data.message && data.message !== '') {
          showError(data.message);
        } else {
          showError('Error :/');
        }
      }
    })
    .catch((error: Error) => {
      console.error(error);
      // HTTP errors carry the server message (like the old parse-then-branch
      // flow); network failures stay generic.
      showError(error instanceof HttpError ? error.message : 'Error :/');
    });
}
