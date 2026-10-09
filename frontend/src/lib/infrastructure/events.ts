import { listen, type UnlistenFn } from "@tauri-apps/api/event";

/**
 * Event Abstraction Layer
 * Wraps Tauri events for testing and cross-platform consistency.
 */

export interface TabletEventData {
    status: string;
    x: number;
    y: number;
    pressure: number;
    hover_distance: number;
    tilt_x: number;
    tilt_y: number;
}

export interface LogEventData {
    time: string;
    level: string;
    group: string;
    message: string;
}

export const events = {
    /** 
     * Subscribes to real-time tablet packets (approx 60Hz)
     */
    async onTabletEvent(callback: (data: TabletEventData) => void): Promise<UnlistenFn | undefined> {
        try {
            if (typeof window !== "undefined" && window.__TAURI_INTERNALS__ !== undefined) {
                return await listen<TabletEventData>("tablet-event", (event) => { callback(event.payload); });
            }
            return undefined;
        } catch (e) {
            console.error("Failed to subscribe to tablet events:", e);
            return undefined;
        }
    },

    /** 
     * Subscribes to new application logs 
     */
    async onLogEvent(callback: (data: LogEventData) => void): Promise<UnlistenFn | undefined> {
        try {
            if (typeof window !== "undefined" && window.__TAURI_INTERNALS__ !== undefined) {
                return await listen<LogEventData>("log-event", (event) => { callback(event.payload); });
            }
            return undefined;
        } catch (e) {
            console.error("Failed to subscribe to log events:", e);
            return undefined;
        }
    },

    /** 
     * Subscribes to device hot-plug (connect/disconnect) events
     */
    async onDeviceChanged(callback: () => void): Promise<UnlistenFn | undefined> {
        try {
            if (typeof window !== "undefined" && window.__TAURI_INTERNALS__ !== undefined) {
                return await listen("device-changed", () => { callback(); });
            }
            return undefined;
        } catch (e) {
            console.error("Failed to subscribe to device changed events:", e);
            return undefined;
        }
    },

    /** 
     * Subscribes to profile loaded events
     */
    async onProfileLoaded(callback: () => void): Promise<UnlistenFn | undefined> {
        try {
            if (typeof window !== "undefined" && window.__TAURI_INTERNALS__ !== undefined) {
                return await listen("profile-loaded", () => { callback(); });
            }
            return undefined;
        } catch (e) {
            console.error("Failed to subscribe to profile loaded events:", e);
            return undefined;
        }
    },

    /**
     * Subscribes to navigate-to-updates events from Tray
     */
    async onNavigateToUpdates(callback: () => void): Promise<UnlistenFn | undefined> {
        try {
            if (typeof window !== "undefined" && window.__TAURI_INTERNALS__ !== undefined) {
                return await listen("navigate-to-updates", () => { callback(); });
            }
            return undefined;
        } catch (e) {
            console.error("Failed to subscribe to navigate-to-updates events:", e);
            return undefined;
        }
    }
};
