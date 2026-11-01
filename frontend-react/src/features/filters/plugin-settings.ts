import type {
  Config,
  DynamicPluginSettings,
  PluginManifest,
  PropertyValue,
} from "@/infrastructure/tauri/commands";

const OFF: DynamicPluginSettings = { enabled: false, properties: {} };

/** What the profile says about a plugin; a plugin it never heard of is disabled. */
export function settingsOf(config: Config, id: string): DynamicPluginSettings {
  return config.plugins[id] ?? OFF;
}

function withSettings(config: Config, id: string, settings: DynamicPluginSettings): Config {
  return { ...config, plugins: { ...config.plugins, [id]: settings } };
}

export function withEnabled(config: Config, id: string, enabled: boolean): Config {
  return withSettings(config, id, { ...settingsOf(config, id), enabled });
}

export function withProperty(
  config: Config,
  id: string,
  key: string,
  value: PropertyValue,
): Config {
  const current = settingsOf(config, id);
  return withSettings(config, id, {
    ...current,
    properties: { ...current.properties, [key]: value },
  });
}

/** The value of every property of a plugin as it ships (a choice is its index). */
export function defaultsOf(plugin: PluginManifest): Record<string, PropertyValue> {
  const values: Record<string, PropertyValue> = {};
  for (const prop of plugin.properties) {
    values[prop.id] =
      prop.kind.type === "Choice" ? prop.kind.config.default_index : prop.kind.config.default;
  }
  return values;
}

export function withDefaults(config: Config, plugin: PluginManifest): Config {
  return withSettings(config, plugin.id, {
    ...settingsOf(config, plugin.id),
    properties: defaultsOf(plugin),
  });
}
