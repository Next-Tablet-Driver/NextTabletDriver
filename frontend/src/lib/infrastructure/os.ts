import { open as shellOpen } from "@tauri-apps/plugin-shell";
import { open as dialogOpen, save as dialogSave, type OpenDialogOptions, type SaveDialogOptions } from "@tauri-apps/plugin-dialog";
import { relaunch as processRelaunch } from "@tauri-apps/plugin-process";

/**
 * OS Abstraction Layer
 * Wraps Tauri plugins (shell, dialog) to make the frontend agnostic and provide centralized error handling.
 */
export const os = {
    /** 
     * Opens a URL in the user's default web browser 
     */
    async openUrl(url: string): Promise<void> {
        try {
            // If we are not in Tauri (e.g. browser environment mock), we could do: window.open(url, "_blank");
            if (typeof window !== "undefined" && window.__TAURI_INTERNALS__ !== undefined) {
                await shellOpen(url);
            } else {
                window.open(url, "_blank");
            }
        } catch (e) {
            console.error(`Failed to open URL ${url}:`, e);
        }
    },
    
    /** 
     * Opens a file selection dialog 
     */
    async openFileDialog(options?: OpenDialogOptions): Promise<string | string[] | null> {
        try {
            if (typeof window !== "undefined" && window.__TAURI_INTERNALS__ !== undefined) {
                return await dialogOpen(options);
            }
            console.warn("openFileDialog is not supported outside of Tauri");
            return null;
        } catch (e) {
            console.error("Failed to open file dialog:", e);
            return null;
        }
    },

    /** 
     * Opens a file save dialog 
     */
    async saveFileDialog(options?: SaveDialogOptions): Promise<string | null> {
        try {
            if (typeof window !== "undefined" && window.__TAURI_INTERNALS__ !== undefined) {
                return await dialogSave(options);
            }
            console.warn("saveFileDialog is not supported outside of Tauri");
            return null;
        } catch (e) {
            console.error("Failed to open save dialog:", e);
            return null;
        }
    },

    /** 
     * Relaunches the application 
     */
    async relaunch(): Promise<void> {
        try {
            if (typeof window !== "undefined" && window.__TAURI_INTERNALS__ !== undefined) {
                await processRelaunch();
            } else {
                window.location.reload();
            }
        } catch (e) {
            console.error("Failed to relaunch application:", e);
        }
    }
};
