const listeners = new Map();
export function on(type, listener) {
    if (!listeners.has(type))
        listeners.set(type, new Set());
    const group = listeners.get(type);
    if (group.size >= 64)
        throw new Error("Listener limit exceeded");
    group.add(listener);
    return () => group.delete(listener);
}
export function emit(event) {
    globalThis.__webdeckEmit(event);
    for (const listener of listeners.get(event.type) || [])
        listener(event);
}
