import { useSyncExternalStore } from "react";

/**
 * A tiny observable value: what `useSyncExternalStore` needs, and nothing else.
 *
 * The state is replaced, never mutated, so a component re-renders exactly when it changes.
 */
export class Store<T> {
  #state: T;
  readonly #listeners = new Set<() => void>();

  constructor(initial: T) {
    this.#state = initial;
  }

  get = (): T => this.#state;

  set(next: T): void {
    if (Object.is(next, this.#state)) {
      return;
    }
    this.#state = next;
    for (const listener of this.#listeners) {
      listener();
    }
  }

  update(change: (current: T) => T): void {
    this.set(change(this.#state));
  }

  subscribe = (listener: () => void): (() => void) => {
    this.#listeners.add(listener);
    return () => {
      this.#listeners.delete(listener);
    };
  };
}

/** Reads a store in a component and re-renders when it changes. */
export function useStore<T>(store: Store<T>): T {
  return useSyncExternalStore(store.subscribe, store.get);
}
