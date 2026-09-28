import type { BootContext, JsonObject } from '../framework/types';

// App lifecycle events. Small processes (boot, refresh, usage poll, editor,
// saves, server link) emit here so unrelated UI, tests, and the demo tour
// can react to state changes instead of sleeping or polling.
//
// Dispatched on `window` as CustomEvents named `webdeck:<name>` so any
// consumer (in-app modules, Playwright, user scripts) can subscribe
// without importing this module.

export interface AppEventMap {
  /** Initial boot finished rendering. */
  'boot:ready': BootContext;
  /** In-place refresh finished rendering. */
  'app:refreshed': BootContext;
  /** A usage poll applied fresh values to the tiles. */
  'usage:updated': JsonObject;
  /** Editor mode flipped (enter/exit). */
  'editor:changed': { mode: number };
  /** A save flow completed (before the resulting refresh). */
  'save:completed': { flow: 'single' | 'add' | 'buttons' | 'config' };
  /** Server unreachable: the reconnect screen is showing. */
  'server:disconnected': { failures: number };
  /** Server reachable again after a disconnect. */
  'server:reconnected': Record<string, never>;
}

export type AppEventName = keyof AppEventMap;

export function emitAppEvent<K extends AppEventName>(name: K, detail: AppEventMap[K]): void {
  window.dispatchEvent(new CustomEvent(`webdeck:${name}`, { detail }));
}

/** Subscribe to an app event; returns an unsubscribe function. */
export function onAppEvent<K extends AppEventName>(
  name: K,
  handler: (detail: AppEventMap[K]) => void
): () => void {
  const listener = (event: Event): void => {
    handler((event as CustomEvent).detail as AppEventMap[K]);
  };
  window.addEventListener(`webdeck:${name}`, listener);
  return () => window.removeEventListener(`webdeck:${name}`, listener);
}
