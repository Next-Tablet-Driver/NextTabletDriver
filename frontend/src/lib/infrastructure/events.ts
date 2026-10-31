import type { UnlistenFn } from "@tauri-apps/api/event";
import { events as backendEvents, type LogEvent, type TabletEvent } from "./tauri/bindings";

/**
 * Event Abstraction Layer
 * Wraps the backend's events for testing and cross-platform consistency. The event names and
 * payload types come from `tauri/bindings.ts`, generated from the Rust code.
 */

export type TabletEventData = TabletEvent;
export type LogEventData = LogEvent;

const insideTauri = (): boolean =>
  typeof window !== "undefined" && window.__TAURI_INTERNALS__ !== undefined;

/** Calls `register` inside Tauri; logs and returns `undefined` otherwise or on failure. */
async function subscribe(
  label: string,
  register: () => Promise<UnlistenFn>,
): Promise<UnlistenFn | undefined> {
  try {
    if (insideTauri()) {
      return await register();
    }
    return undefined;
  } catch (e) {
    console.error(`Failed to subscribe to ${label} events:`, e);
    return undefined;
  }
}

export const events = {
  /** Subscribes to real-time tablet packets (approx 60Hz) */
  onTabletEvent(callback: (data: TabletEventData) => void): Promise<UnlistenFn | undefined> {
    return subscribe("tablet", () =>
      backendEvents.tabletEvent.listen((event) => {
        callback(event.payload);
      }),
    );
  },

  /** Subscribes to new application logs */
  onLogEvent(callback: (data: LogEventData) => void): Promise<UnlistenFn | undefined> {
    return subscribe("log", () =>
      backendEvents.logEvent.listen((event) => {
        callback(event.payload);
      }),
    );
  },

  /** Subscribes to device hot-plug (connect/disconnect) events */
  onDeviceChanged(callback: () => void): Promise<UnlistenFn | undefined> {
    return subscribe("device changed", () =>
      backendEvents.deviceChanged.listen(() => {
        callback();
      }),
    );
  },

  /** Subscribes to profile loaded events */
  onProfileLoaded(callback: () => void): Promise<UnlistenFn | undefined> {
    return subscribe("profile loaded", () =>
      backendEvents.profileLoaded.listen(() => {
        callback();
      }),
    );
  },

  /** Subscribes to navigate-to-updates events from Tray */
  onNavigateToUpdates(callback: () => void): Promise<UnlistenFn | undefined> {
    return subscribe("navigate-to-updates", () =>
      backendEvents.navigateToUpdates.listen(() => {
        callback();
      }),
    );
  },
};
