import { useCallback, useEffect, useRef, useState } from "react";

/** What a page shows for a call to the backend: nothing yet, the value, or why it failed. */
export type CommandState<T> =
  { status: "loading" } | { status: "ready"; data: T } | { status: "error"; message: string };

/**
 * Calls a backend command when the component mounts (and again on `reload`).
 *
 * The result of a call that is no longer the latest, or that finishes after unmount, is dropped.
 * `call` must be stable (a function of `infrastructure/tauri/commands.ts`, or wrapped in
 * `useCallback`), because a new one starts a new call.
 */
export function useCommand<T>(call: () => Promise<T>): {
  state: CommandState<T>;
  reload: () => void;
} {
  const [state, setState] = useState<CommandState<T>>({ status: "loading" });
  const attempt = useRef(0);

  const run = useCallback(() => {
    const mine = ++attempt.current;
    call().then(
      (data) => {
        if (mine === attempt.current) {
          setState({ status: "ready", data });
        }
      },
      (error: unknown) => {
        if (mine === attempt.current) {
          const message = error instanceof Error ? error.message : String(error);
          setState({ status: "error", message });
        }
      },
    );
  }, [call]);

  useEffect(() => {
    const counter = attempt;
    run();
    return () => {
      // Whatever is still pending belongs to a component that is gone, or to an older `call`.
      counter.current++;
    };
  }, [run]);

  return { state, reload: run };
}
