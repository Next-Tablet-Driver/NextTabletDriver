declare global {
  interface Window {
    __TAURI_INTERNALS__?: Record<string, unknown>;
    __TAURI_EVENT_PLUGIN_INTERNALS__?: Record<string, unknown>;
    /** Every IPC call made while the Tauri mock is installed (development and end-to-end tests only). */
    __ntdInvocations?: { command: string; args: Record<string, unknown> }[];
  }
}

export {};
