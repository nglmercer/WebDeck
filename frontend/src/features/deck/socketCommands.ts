import { showAlert } from '../../components/dialog';
import type { CommandEvent, CommandRequest } from '../../contracts/v2';

/** Correlation only: request identifiers never authorize replay. */
export function commandRequestId(): string {
  // getRandomValues also works for paired clients on ordinary LAN HTTP.
  return Array.from(crypto.getRandomValues(new Uint8Array(16)), byte => byte.toString(16).padStart(2, '0')).join('');
}

export class SocketCommandTracker {
  private pending = new Map<string, ReturnType<typeof setTimeout>>();
  constructor(private report: (message: string) => void, private timeout = 35000) {}
  track(request: CommandRequest): void {
    const id = request.request_id;
    if (!id || this.pending.has(id)) throw new Error('A unique request identifier is required');
    this.pending.set(id, setTimeout(() => {
      this.pending.delete(id);
      this.report('Command response timed out. Its outcome is unknown; it will not be retried.');
    }, this.timeout));
  }
  result(event: CommandEvent): void {
    const timer = this.pending.get(event.request_id);
    if (timer === undefined || event.state === 'accepted') return;
    clearTimeout(timer); this.pending.delete(event.request_id);
    if (event.state === 'failed') this.report(event.message);
  }
  disconnect(): void {
    if (this.pending.size) this.report('Disconnected while a command was pending. Its outcome is unknown; it will not be retried.');
    for (const timer of this.pending.values()) clearTimeout(timer);
    this.pending.clear();
  }
}

export const socketCommandTracker = new SocketCommandTracker(message => { void showAlert(message); });
