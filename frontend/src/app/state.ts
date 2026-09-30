import type { Socket } from 'socket.io-client';
import type { JsonObject } from '../framework/types';

// Shared mutable page state (was implicit globals in the inline script).

export const pageState = {
  canEdit: true,
  config: {} as JsonObject,
  tempEditorConfig: {} as JsonObject,
  editorMode: 0,
  disconnectCount: 0,
};

/** Server -> client events (must match `socketio_layer` in src/app/server/realtime.rs). */
interface ServerToClientEvents {
  command_result: (event: import('../contracts/v2').CommandEvent) => void;
}

interface ClientToServerEvents {
  command: (request: import('../contracts/v2').CommandRequest) => void;
}



export type AppSocket = Socket<ServerToClientEvents, ClientToServerEvents>;

export const socketHolder: { socket: AppSocket | null } = { socket: null };
