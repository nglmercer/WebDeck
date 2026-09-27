import type { JsonObject } from '../framework/types';

// Shared mutable page state (was implicit globals in the inline script).

export const pageState = {
  config: {} as JsonObject,
  tempEditorConfig: {} as JsonObject,
  editorMode: 0,
  disconnectCount: 0,
};

export interface SocketLike {
  on: (event: string, cb: (data?: unknown) => void) => void;
  emit: (event: string, data?: unknown) => void;
}

export const socketHolder: { socket: SocketLike | null } = { socket: null };
