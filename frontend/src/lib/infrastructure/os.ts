import { open as shellOpen } from "@tauri-apps/plugin-shell";
import {
  confirm as dialogConfirm,
  message as dialogMessage,
  open as dialogOpen,
  save as dialogSave,
  type ConfirmDialogOptions,
  type MessageDialogOptions,
  type OpenDialogOptions,
  type SaveDialogOptions,
} from "@tauri-apps/plugin-dialog";
import { relaunch as processRelaunch } from "@tauri-apps/plugin-process";
import type { Update } from "@tauri-apps/plugin-updater";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import { log } from "./log";

export type { Update };

/** Options of a second window, as `WebviewWindow` takes them. */
export type WindowOptions = NonNullable<ConstructorParameters<typeof WebviewWindow>[1]>;

/**
 * OS Abstraction Layer
 * Wraps Tauri plugins (shell, dialog, updater, process) to make the frontend agnostic and provide
 * centralized error handling.
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
      log.error(`Failed to open URL ${url}:`, e);
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
      log.warn("openFileDialog is not supported outside of Tauri");
      return null;
    } catch (e) {
      log.error("Failed to open file dialog:", e);
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
      log.warn("saveFileDialog is not supported outside of Tauri");
      return null;
    } catch (e) {
      log.error("Failed to open save dialog:", e);
      return null;
    }
  },

  /** Asks a yes/no question in a native dialog. Rejects when the dialog cannot be shown. */
  confirm(text: string, options?: string | ConfirmDialogOptions): Promise<boolean> {
    return dialogConfirm(text, options);
  },

  /** Shows a message in a native dialog. Rejects when the dialog cannot be shown. */
  async message(text: string, options?: string | MessageDialogOptions): Promise<void> {
    await dialogMessage(text, options);
  },

  /**
   * Looks for a newer version of the application. Resolves to `null` when this one is the latest.
   * The updater is loaded on first use, so a normal start does not pay for it.
   */
  async checkForUpdate(): Promise<Update | null> {
    const { check } = await import("@tauri-apps/plugin-updater");
    return check();
  },

  /** Opens a second window of the application. */
  openWindow(label: string, options: WindowOptions): void {
    new WebviewWindow(label, options);
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
      log.error("Failed to relaunch application:", e);
    }
  },
};
