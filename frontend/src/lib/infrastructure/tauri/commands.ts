import { invoke } from "@tauri-apps/api/core";

export interface Area {
    w: number;
    h: number;
    x: number;
    y: number;
}

export interface ActiveArea extends Area {
    rotation: number;
}

export interface Config {
    target_area: Area;
    active_area: ActiveArea;
    display_snapping: boolean;
    lock_aspect_ratio: boolean;
    show_osu_playfield: boolean;
    mode: string;
    plugins?: Record<string, DynamicPluginSettings>;
    tip_threshold: number;
    eraser_threshold: number;
    disable_pressure: boolean;
    disable_tilt: boolean;
    tip_binding: string;
    eraser_binding: string;
    pen_button_bindings: string[];
    run_at_startup: boolean;
    system_tray_on_minimize: boolean;
    force_high_resolution_timer: boolean;
    websocket: {
        enabled: boolean;
        port: number;
        polling_rate_hz: number;
        send_coordinates: boolean;
        send_pressure: boolean;
        send_tilt: boolean;
        send_status: boolean;
    };
    pressure_curve: {
        curve_type: 'Linear' | 'Exponential' | 'Custom';
        exponent: number;
        points: [number, number][];
    };
    theme: string;
    language: string;
}

export type PropertyValue = number | boolean | string;

export interface DynamicPluginSettings {
    enabled: boolean;
    properties: Record<string, PropertyValue>;
}

export type PluginPropertyKind = 
    | { type: 'Float' | 'Int', config: { min: number, max: number, step: number, default: number, unit?: string } }
    | { type: 'Bool', config: { default: boolean } }
    | { type: 'String', config: { default: string } }
    | { type: 'Choice', config: { options: string[], default_index: number } };

export interface PropertyDescriptor {
    id: string;
    name: string;
    tooltip: string | null;
    kind: PluginPropertyKind;
}

export interface PluginManifest {
    id: string;
    name: string;
    version: string;
    author: string;
    description: string;
    icon: string | null;
    properties: PropertyDescriptor[];
}

export interface TabletStatus {
    connected: boolean;
    name: string;
    width: number;
    height: number;
    max_x?: number;
    max_y?: number;
}

export interface EngineMetrics {
    hz: number;
    last_packet_count: number;
}

export interface PresetInfo {
    name: string;
    path: string;
}

export async function getCoreVersion(): Promise<string> {
    return invoke<string>("get_core_version");
}

export async function getTabletStatus(): Promise<TabletStatus> {
    return invoke<TabletStatus>("get_tablet_status");
}

export async function getConfig(): Promise<Config> {
    return invoke<Config>("get_config");
}

export async function setConfig(newConfig: Config): Promise<void> {
    return invoke<undefined>("set_config", { newConfig });
}

export async function saveConfig(): Promise<void> {
    return invoke<undefined>("save_config");
}

export async function getMetrics(): Promise<EngineMetrics> {
    return invoke<EngineMetrics>("get_metrics");
}

export async function resetConfig(): Promise<void> {
    return invoke<undefined>("reset_config");
}

export async function loadProfile(path: string): Promise<void> {
    return invoke<undefined>("load_profile", { path });
}

export async function importOtdProfile(path: string): Promise<void> {
    return invoke<undefined>("import_otd_profile", { path });
}

export async function exportProfile(path: string): Promise<void> {
    return invoke<undefined>("export_profile", { path });
}

export async function getPresets(): Promise<PresetInfo[]> {
    return invoke<PresetInfo[]>("get_presets");
}

export async function getCurrentProfileName(): Promise<string> {
    return invoke<string>("get_current_profile_name");
}

export interface LogEntry {
    time: string;
    level: string;
    group: string;
    message: string;
    search_text: string;
}

export async function getLogs(): Promise<LogEntry[]> {
    if (typeof window !== "undefined" && window.__TAURI_INTERNALS__ === undefined) return [];
    return invoke<LogEntry[]>("get_logs");
}

export async function clearLogs(): Promise<void> {
    if (typeof window !== "undefined" && window.__TAURI_INTERNALS__ === undefined) return;
    return invoke<undefined>("clear_logs");
}

export async function getAvailablePlugins(): Promise<PluginManifest[]> {
    return invoke<PluginManifest[]>("get_available_plugins");
}

export async function reloadPlugins(): Promise<PluginManifest[]> {
    return invoke<PluginManifest[]>("reload_plugins");
}

export async function openPluginsFolder(): Promise<void> {
    return invoke<undefined>("open_plugins_folder");
}

/** Opens a native picker + confirmation in the backend. Resolves to `false` if cancelled. */
export async function installPlugin(): Promise<boolean> {
    return invoke<boolean>("install_plugin");
}

/** A plugin library found in the plugins folder that is not trusted and was not loaded. */
export interface UntrustedPlugin {
    file_name: string;
    sha256: string;
}

export async function getUntrustedPlugins(): Promise<UntrustedPlugin[]> {
    return invoke<UntrustedPlugin[]>("get_untrusted_plugins");
}

/** Asks the user (natively) to trust the plugin with this hash. Resolves to `false` if declined. */
export async function trustPlugin(sha256: string): Promise<boolean> {
    return invoke<boolean>("trust_plugin", { sha256 });
}

export async function deletePlugin(id: string): Promise<void> {
    return invoke<undefined>("delete_plugin", { id });
}

/** A user theme file as read by the backend (validated by the frontend, see `lib/theme`). */
export interface ThemeFile {
    id: string;
    file_name: string;
    content: string;
}

export async function listThemes(): Promise<ThemeFile[]> {
    return invoke<ThemeFile[]>("list_themes");
}

/** Opens a native file picker and imports the chosen theme. Resolves to `null` if cancelled. */
export async function importTheme(): Promise<ThemeFile | null> {
    return invoke<ThemeFile | null>("import_theme");
}

export async function deleteTheme(id: string): Promise<void> {
    return invoke<undefined>("delete_theme", { id });
}

export async function openThemesFolder(): Promise<void> {
    return invoke<undefined>("open_themes_folder");
}
