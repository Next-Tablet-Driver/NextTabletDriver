import { getCurrentWindow, availableMonitors, type Monitor, PhysicalSize, PhysicalPosition } from "@tauri-apps/api/window";

export type { Monitor };

/**
 * Window Abstraction Layer
 * Wraps Tauri window API for testing and cross-platform consistency.
 */
export const windowControls = {
    async minimize(): Promise<void> {
        try {
            if (typeof window !== "undefined" && window.__TAURI_INTERNALS__ !== undefined) {
                await getCurrentWindow().minimize();
            }
        } catch (e) {
            console.error("Failed to minimize window:", e);
        }
    },
    
    async hide(): Promise<void> {
        try {
            if (typeof window !== "undefined" && window.__TAURI_INTERNALS__ !== undefined) {
                await getCurrentWindow().hide();
            }
        } catch (e) {
            console.error("Failed to hide window:", e);
        }
    },
    
    async toggleMaximize(): Promise<void> {
        try {
            if (typeof window !== "undefined" && window.__TAURI_INTERNALS__ !== undefined) {
                await getCurrentWindow().toggleMaximize();
            }
        } catch (e) {
            console.error("Failed to toggle maximize:", e);
        }
    },
    
    async close(): Promise<void> {
        try {
            if (typeof window !== "undefined" && window.__TAURI_INTERNALS__ !== undefined) {
                await getCurrentWindow().close();
            } else {
                window.close();
            }
        } catch (e) {
            console.error("Failed to close window:", e);
        }
    },
    
    async getMonitors(): Promise<Monitor[]> {
        try {
            if (typeof window !== "undefined" && window.__TAURI_INTERNALS__ !== undefined) {
                return await availableMonitors();
            }
            // Mock monitor for browser development
            return [{
                name: "Mock Monitor",
                size: new PhysicalSize(1920, 1080),
                position: new PhysicalPosition(0, 0),
                scaleFactor: 1,
                workArea: {
                    position: new PhysicalPosition(0, 0),
                    size: new PhysicalSize(1920, 1080)
                }
            }];
        } catch (e) {
            console.error("Failed to get monitors:", e);
            return [];
        }
    }
};
