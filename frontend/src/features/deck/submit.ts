import { showAlert } from '../../components/dialog';
import { fetchUsage } from '../../api/usage';
import { send_data } from '../../app/send';
import { isSwapMode } from '../../app/editor';
import { pageState, socketHolder } from '../../app/state';
import { updateUsageTiles } from '../../app/usage';

/** Owned by Grid's form event, with no global DOM submit listener. */
export function submitDeck(event: SubmitEvent, transfer: string): void {
  event.preventDefault();
  if (pageState.editorMode === 1 && isSwapMode()) return;
  const form = event.currentTarget as HTMLFormElement;
  const message = form.querySelector<HTMLInputElement>('.message')?.value ?? '';
  if (message.startsWith('/reload') && !isSwapMode()) { location.reload(); return; }
  if (message.startsWith('/folder')) return;
  if (message.startsWith('/usage')) {
    void fetchUsage({ message }).then(updateUsageTiles).catch(console.error);
  } else if (transfer === 'socket') {
    // No replay on reconnect: native side effects are not idempotent.
    if (!socketHolder.socket?.connected) { void showAlert('Disconnected. The command was not sent; reconnect before trying again.'); return; }
    socketHolder.socket.emit('message_from_socket', message);
  } else { send_data(message); }
}
