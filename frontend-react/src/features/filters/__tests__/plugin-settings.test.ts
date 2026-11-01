import { describe, expect, it } from "vitest";
import {
  defaultsOf,
  settingsOf,
  withDefaults,
  withEnabled,
  withProperty,
} from "@/features/filters/plugin-settings";
import type { Config, PluginManifest } from "@/infrastructure/tauri/commands";

const config = { plugins: {} } as unknown as Config;

const plugin = {
  id: "smooth",
  name: "Smooth",
  version: "1.0.0",
  author: "Someone",
  description: "",
  icon: null,
  target: "",
  properties: [
    {
      id: "radius",
      name: "Radius",
      tooltip: null,
      kind: { type: "Float", config: { min: 0, max: 10, step: 0.1, unit: "px", default: 2.5 } },
    },
    {
      id: "mode",
      name: "Mode",
      tooltip: null,
      kind: { type: "Choice", config: { options: ["a", "b"], default_index: 1 } },
    },
    {
      id: "on",
      name: "On",
      tooltip: null,
      kind: { type: "Bool", config: { default: true } },
    },
    {
      id: "label",
      name: "Label",
      tooltip: null,
      kind: { type: "String", config: { default: "x" } },
    },
  ],
} as unknown as PluginManifest;

describe("plugin settings", () => {
  it("treats an unknown plugin as disabled", () => {
    expect(settingsOf(config, "smooth")).toEqual({ enabled: false, properties: {} });
  });

  it("enables a plugin without touching the others or the input", () => {
    const next = withEnabled(config, "smooth", true);
    expect(settingsOf(next, "smooth").enabled).toBe(true);
    expect(config.plugins).toEqual({});
  });

  it("sets one property and keeps the rest", () => {
    const once = withProperty(config, "smooth", "radius", 4);
    const twice = withProperty(once, "smooth", "on", false);
    expect(settingsOf(twice, "smooth").properties).toEqual({ radius: 4, on: false });
  });

  it("knows the defaults of every kind of property", () => {
    expect(defaultsOf(plugin)).toEqual({ radius: 2.5, mode: 1, on: true, label: "x" });
  });

  it("resets the properties and keeps the plugin enabled", () => {
    const edited = withProperty(withEnabled(config, "smooth", true), "smooth", "radius", 9);
    const reset = withDefaults(edited, plugin);
    expect(settingsOf(reset, "smooth")).toEqual({
      enabled: true,
      properties: { radius: 2.5, mode: 1, on: true, label: "x" },
    });
  });
});
