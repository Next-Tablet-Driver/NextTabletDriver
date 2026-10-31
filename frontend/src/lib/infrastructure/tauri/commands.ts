/**
 * The backend's commands, as plain functions.
 *
 * The types and the command names come from `bindings.ts`, which is generated from the Rust code
 * (see docs/development.md). This file adds no type of its own: it only keeps the calls simple
 * (a failed command throws its message) and tolerates running outside Tauri.
 */
import {
  commands,
  type ActiveArea,
  type DriverMode,
  type EngineMetrics,
  type LogEntry,
  type TabletStatus,
  type PluginPropertyValue,
  type PresetInfo,
  type PropertyKind,
  type TargetArea,
  type ThemeFile,
  type UntrustedPlugin,
  type DynamicPluginSettings_Serialize,
  type MappingConfig_Serialize,
  type PluginManifest_Serialize,
  type PropertyDescriptor_Serialize,
  type Release_Serialize,
} from "./bindings";

export type {
  ActiveArea,
  DriverMode,
  EngineMetrics,
  LogEntry,
  PresetInfo,
  TabletStatus,
  ThemeFile,
  UntrustedPlugin,
};

/** The mapping and application settings, as the backend returns them. */
export type Config = MappingConfig_Serialize;
/** A rectangle on the screen. */
export type Area = TargetArea;
export type DynamicPluginSettings = DynamicPluginSettings_Serialize;
export type PropertyValue = PluginPropertyValue;
export type PluginPropertyKind = PropertyKind;
export type PropertyDescriptor = PropertyDescriptor_Serialize;
export type PluginManifest = PluginManifest_Serialize;
/** A GitHub release of the project. */
export type Release = Release_Serialize;

type Result<T, E> = { status: "ok"; data: T } | { status: "error"; error: E };

/**
 * Turns a command's `Result` back into a promise that rejects with the backend's message, which
 * is how the interface has always handled a failed command.
 */
async function unwrap<T>(call: Promise<Result<T, string>>): Promise<T> {
  const result = await call;
  if (result.status === "error") {
    // eslint-disable-next-line @typescript-eslint/only-throw-error -- callers read the message itself
    throw result.error;
  }
  return result.data;
}

const outsideTauri = (): boolean =>
  typeof window !== "undefined" && window.__TAURI_INTERNALS__ === undefined;

export async function getCoreVersion(): Promise<string> {
  return commands.getCoreVersion();
}

export async function getTabletStatus(): Promise<TabletStatus> {
  return commands.getTabletStatus();
}

export async function getConfig(): Promise<Config> {
  return commands.getConfig();
}

export async function setConfig(newConfig: Config): Promise<void> {
  return commands.setConfig(newConfig);
}

export async function saveConfig(): Promise<void> {
  return commands.saveConfig();
}

export async function getMetrics(): Promise<EngineMetrics> {
  return commands.getMetrics();
}

/**
 * Checks a button binding with the driver and returns it in the plain spelling (`ctrl+z` becomes
 * `Ctrl+Z`). Rejects with a readable message when the text is not a binding.
 */
export async function normalizeBinding(text: string): Promise<string> {
  return unwrap(commands.normalizeBinding(text));
}

export async function resetConfig(): Promise<void> {
  return commands.resetConfig();
}

export async function loadProfile(path: string): Promise<void> {
  await unwrap(commands.loadProfile(path));
}

export async function importOtdProfile(path: string): Promise<void> {
  await unwrap(commands.importOtdProfile(path));
}

export async function exportProfile(path: string): Promise<void> {
  await unwrap(commands.exportProfile(path));
}

export async function getPresets(): Promise<PresetInfo[]> {
  return commands.getPresets();
}

export async function getCurrentProfileName(): Promise<string> {
  return commands.getCurrentProfileName();
}

export async function getLogs(): Promise<LogEntry[]> {
  if (outsideTauri()) return [];
  return commands.getLogs();
}

export async function clearLogs(): Promise<void> {
  if (outsideTauri()) return;
  return commands.clearLogs();
}

/** Fetches the releases of the project from GitHub. Rejects with a message when GitHub is unreachable. */
export async function getReleases(): Promise<Release[]> {
  return unwrap(commands.getReleases());
}

export async function getAvailablePlugins(): Promise<PluginManifest[]> {
  return commands.getAvailablePlugins();
}

export async function reloadPlugins(): Promise<PluginManifest[]> {
  return commands.reloadPlugins();
}

export async function openPluginsFolder(): Promise<void> {
  await unwrap(commands.openPluginsFolder());
}

/** Opens a native picker + confirmation in the backend. Resolves to `false` if cancelled. */
export async function installPlugin(): Promise<boolean> {
  return unwrap(commands.installPlugin());
}

export async function getUntrustedPlugins(): Promise<UntrustedPlugin[]> {
  return commands.getUntrustedPlugins();
}

/** Asks the user (natively) to trust the plugin with this hash. Resolves to `false` if declined. */
export async function trustPlugin(sha256: string): Promise<boolean> {
  return unwrap(commands.trustPlugin(sha256));
}

export async function deletePlugin(id: string): Promise<void> {
  await unwrap(commands.deletePlugin(id));
}

export async function listThemes(): Promise<ThemeFile[]> {
  return commands.listThemes();
}

/** Opens a native file picker and imports the chosen theme. Resolves to `null` if cancelled. */
export async function importTheme(): Promise<ThemeFile | null> {
  return unwrap(commands.importTheme());
}

export async function deleteTheme(id: string): Promise<void> {
  await unwrap(commands.deleteTheme(id));
}

export async function openThemesFolder(): Promise<void> {
  await unwrap(commands.openThemesFolder());
}
