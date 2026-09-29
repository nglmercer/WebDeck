import type { Socket } from 'socket.io-client';
import type { JsonObject } from '../framework/types';

// Shared mutable page state (was implicit globals in the inline script).

export const pageState = {
  config: {} as JsonObject,
  tempEditorConfig: {} as JsonObject,
  editorMode: 0,
  disconnectCount: 0,
};

/** Server -> client events (must match `socketio_layer` in src/app/server/realtime.rs). */
interface ServerToClientEvents {
  /** Echo of the original command string after it ran. */
  json_data: (message: string) => void;
  /** Broadcast from the `send` handler. */
  message: (data: unknown) => void;
}

/** Client -> server events (must match `socketio_layer` in src/app/server/realtime.rs). */
interface ClientToServerEvents {
  message_from_socket: (message: string) => void;
  send: (data: unknown) => void;
}

export type AppSocket = Socket<ServerToClientEvents, ClientToServerEvents>;

export const socketHolder: { socket: AppSocket | null } = { socket: null };
