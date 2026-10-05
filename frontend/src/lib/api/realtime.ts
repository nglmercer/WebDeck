import { io, type Socket } from 'socket.io-client';
import type { Command, CommandEvent, RuntimeEvent } from '../contracts';
import { contract } from '../schema';
import { credential, request } from './http';
import { id } from '../id';
const eventListeners = new Set<(event: RuntimeEvent) => void>();
export function onRuntimeEvent(listener: (event: RuntimeEvent) => void) {
  eventListeners.add(listener);
  return () => {
    eventListeners.delete(listener);
  };
}
const listeners = new Set<(connected: boolean) => void>();
export function onConnectionChange(listener: (connected: boolean) => void) {
  listeners.add(listener);
  listener(!!socket?.connected);
  return () => {
    listeners.delete(listener);
  };
}
function reportConnection(connected: boolean) {
  listeners.forEach((listener) => listener(connected));
}
let socket: Socket | undefined;
type Pending = {
  resolve: (event: CommandEvent) => void;
  reject: (e: Error) => void;
  timer: ReturnType<typeof setTimeout>;
};
const pending = new Map<string, Pending>();
export function disconnect() {
  socket?.disconnect();
  socket = undefined;
  reportConnection(false);
  for (const p of pending.values()) {
    clearTimeout(p.timer);
    p.reject(new Error('Connection lost; execution outcome is unknown. No retry was sent.'));
  }
  pending.clear();
}
export function connect() {
  if (socket) return;
  socket = io('/v2', { auth: credential() ? { token: credential() } : {}, autoConnect: false });
  socket.on('connect', () => reportConnection(true));
  socket.on('connect_error', () => reportConnection(false));
  socket.on('runtime_event', (value: unknown) => {
    try {
      const event = contract<RuntimeEvent>('RuntimeEvent', value);
      eventListeners.forEach((listener) => listener(event));
    } catch {
      /* Ignore malformed events. */
    }
  });
  socket.on('command_result', (v: unknown) => {
    let event: CommandEvent;
    try {
      event = contract('CommandEvent', v);
    } catch {
      return;
    }
    const p = pending.get(event.request_id);
    if (!p || event.state === 'accepted') return;
    clearTimeout(p.timer);
    pending.delete(event.request_id);
    p.resolve(event);
  });
  socket.on('disconnect', () => {
    reportConnection(false);
    for (const p of pending.values()) {
      clearTimeout(p.timer);
      p.reject(new Error('Connection lost; execution outcome is unknown. No retry was sent.'));
    }
    pending.clear();
  });
  socket.connect();
}
export async function execute(
  command: Command,
  transport: 'http' | 'socket',
): Promise<CommandEvent> {
  const request_id = id();
  const body = { request_id, command };
  contract('CommandRequest', body);
  if (transport === 'http')
    return request('commands', 'CommandEvent', { method: 'POST', body: JSON.stringify(body) });
  connect();
  if (!socket?.connected) throw new Error('Realtime connection is offline');
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => {
      pending.delete(request_id);
      reject(new Error('Result timed out; execution outcome is unknown. No retry was sent.'));
    }, 35000);
    pending.set(request_id, { resolve, reject, timer });
    socket?.volatile.emit('command', body);
  });
}
