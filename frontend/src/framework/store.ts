// Tiny observable store. Views render from snapshots; this only notifies.

export type Listener<T> = (value: T) => void;

export class Store<T> {
  private value: T;
  private listeners = new Set<Listener<T>>();

  constructor(initial: T) {
    this.value = initial;
  }

  get(): T {
    return this.value;
  }

  set(next: T): void {
    this.value = next;
    for (const listener of this.listeners) listener(next);
  }

  update(fn: (value: T) => T): void {
    this.set(fn(this.value));
  }

  subscribe(listener: Listener<T>): () => void {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  }
}
