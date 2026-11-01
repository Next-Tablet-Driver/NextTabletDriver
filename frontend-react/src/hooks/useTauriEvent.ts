import { useEffect, useRef } from "react";
import type { UnlistenFn } from "@tauri-apps/api/event";

/** Any `events.onXxx` function of `infrastructure/tauri/events.ts`. */
type Subscribe<T> = (callback: (data: T) => void) => Promise<UnlistenFn | undefined>;

/**
 * Subscribes to a backend event for as long as the component is mounted.
 *
 * `handler` may change on every render without re-subscribing: the latest one is called. The
 * subscription is released even when the component unmounts before `subscribe` has resolved.
 */
export function useTauriEvent<T>(subscribe: Subscribe<T>, handler: (data: T) => void): void {
  const latest = useRef(handler);
  useEffect(() => {
    latest.current = handler;
  });

  useEffect(() => {
    let cancelled = false;
    let unlisten: UnlistenFn | undefined;
    void subscribe((data) => {
      latest.current(data);
    }).then((fn) => {
      if (cancelled) {
        fn?.();
      } else {
        unlisten = fn;
      }
    });
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [subscribe]);
}
