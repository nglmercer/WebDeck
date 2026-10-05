const listeners = new Map<string, Set<(event: unknown) => void>>();
export function on(type: string, listener: (event: unknown) => void) {
  if (!listeners.has(type)) listeners.set(type, new Set());
  const group = listeners.get(type)!;
  if (group.size >= 64) throw new Error("Listener limit exceeded");
  group.add(listener);
  return () => group.delete(listener);
}
export function emit(event: {
  api_version: 2;
  type: string;
  [key: string]: unknown;
}) {
  (globalThis as any).__webdeckEmit(event);
  for (const listener of listeners.get(event.type) || []) listener(event);
}
