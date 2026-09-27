import { postJson } from '../framework/api';
import type { JsonObject } from '../framework/types';
import { showError } from './toast';

// Port of send_data() (also exposed as window.send_data for inline onclick).
export function send_data(message: string): void {
  fetch('/send-data', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify({ message }),
  })
    .then((response) => response.json())
    .then((data: { success?: boolean; message?: string }) => {
      if (data.success) {
        console.log(data.message);
      } else {
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
      showError('Error :/');
    });
}

export async function loadConfig(): Promise<JsonObject> {
  return postJson<JsonObject>('/get_config', {});
}
