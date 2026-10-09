/**
 * Theming entry point.
 *
 * `themeStore` is the single instance that decides which theme is on screen; the rest of the
 * app only calls `themeStore.select(...)` and reads its state.
 */
import { listThemes } from "../infrastructure/tauri/commands";
import { ThemeStore } from "./theme-store.svelte";

export const themeStore = new ThemeStore();

/**
 * Reads the theme files from disk (through the backend) and hands them to the store.
 * Outside the desktop app, or if the backend call fails, there are simply no user themes.
 */
export async function refreshUserThemes(): Promise<void> {
    try {
        const files = await listThemes();
        themeStore.setUserThemes(files.map((f) => ({ id: f.id, fileName: f.file_name, content: f.content })));
    } catch (e) {
        console.error("Failed to load user themes:", e);
        themeStore.setUserThemes([]);
    }
}

export { BUILTIN_THEMES, CUSTOM_PREFIX, SYSTEM_THEME } from "./apply";
export { buildThemeTemplate, documentTokenReader } from "./template";
export type { UserThemeEntry, UserThemeSource } from "./theme-store.svelte";
export type { ThemeDefinition, ValidationIssue } from "./validate";
