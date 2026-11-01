import { Info, RefreshCw, Trash } from "lucide-react";
import { PluginIcon } from "@/features/filters/FiltersSidebar";
import {
  settingsOf,
  withDefaults,
  withEnabled,
  withProperty,
} from "@/features/filters/plugin-settings";
import { useMappingSnapshot, useMappingStore } from "@/features/mapping/MappingProvider";
import type {
  PluginManifest,
  PropertyDescriptor,
  PropertyValue,
} from "@/infrastructure/tauri/commands";
import { Button } from "@/ui/Button";
import { Checkbox } from "@/ui/Checkbox";
import { classes } from "@/ui/classes";
import { Input } from "@/ui/Input";
import { Select } from "@/ui/Select";
import styles from "@/features/filters/PluginDetails.module.css";

function PropertyInput({
  prop,
  value,
  onChange,
}: {
  prop: PropertyDescriptor;
  value: PropertyValue | undefined;
  onChange: (value: PropertyValue) => void;
}) {
  const { kind } = prop;
  switch (kind.type) {
    case "Float":
    case "Int": {
      const { config } = kind;
      return (
        <div className={styles["number-input-wrapper"]}>
          <Input
            type="number"
            max={config.max ?? undefined}
            min={config.min ?? undefined}
            step={config.step ?? undefined}
            unit={config.unit}
            value={typeof value === "number" ? value : (config.default ?? 0)}
            onCommit={onChange}
          />
        </div>
      );
    }
    case "Bool":
      return (
        <Checkbox
          checked={typeof value === "boolean" ? value : kind.config.default}
          onChange={onChange}
          aria-label={prop.name}
        />
      );
    case "Choice":
      return (
        <div className={styles["theme-select-wrapper"]}>
          <Select
            value={typeof value === "number" ? value : kind.config.default_index}
            options={kind.config.options.map((label, index) => ({ value: index, label }))}
            onChange={onChange}
          />
        </div>
      );
    case "String":
      return (
        <Input
          type="text"
          aria-label={prop.name}
          value={typeof value === "string" ? value : kind.config.default}
          onText={onChange}
        />
      );
  }
  return null;
}

/** What a filter is, whether it is on, and its settings. */
export function PluginDetails({
  plugin,
  onDelete,
}: {
  plugin: PluginManifest;
  onDelete: () => void;
}) {
  const { config } = useMappingSnapshot();
  const mapping = useMappingStore();
  if (config === null) {
    return null;
  }
  const settings = settingsOf(config, plugin.id);

  return (
    <>
      <div className={styles["plugin-header-area"]}>
        <div className={styles["plugin-title-row"]}>
          <span className={styles["header-icon"]}>
            <PluginIcon pluginId={plugin.id} size={24} />
          </span>
          <h2>{plugin.name}</h2>
          <span className={styles.badge}>v{plugin.version}</span>
        </div>
        <div className={styles["plugin-by-row"]}>By {plugin.author}</div>
        <div className={styles["plugin-desc-row"]}>
          <div className={styles["desc-text"]}>{plugin.description}</div>
          <div className={styles["plugin-actions"]}>
            <Checkbox
              label="Enable Plugin"
              checked={settings.enabled}
              onChange={(enabled) => {
                mapping.update((c) => withEnabled(c, plugin.id, enabled));
              }}
            />
            <Button variant="danger-ghost" onClick={onDelete}>
              <Trash size={12} /> Delete this plugin
            </Button>
          </div>
        </div>
      </div>

      <div className={styles["properties-section"]}>
        <h3>Settings</h3>
        <div className={classes(styles["properties-list"], !settings.enabled && styles.disabled)}>
          {plugin.properties.map((prop) => (
            <div key={prop.id} className={styles["property-row"]}>
              <div className={styles["prop-label-wrapper"]}>
                <span className={styles["prop-name"]}>{prop.name}</span>
                {prop.tooltip !== null && prop.tooltip !== "" && (
                  <span className={styles["info-icon"]} title={prop.tooltip}>
                    <Info size={12} />
                  </span>
                )}
              </div>
              <div className={styles["prop-input"]}>
                <PropertyInput
                  prop={prop}
                  value={settings.properties[prop.id]}
                  onChange={(value) => {
                    mapping.update((c) => withProperty(c, plugin.id, prop.id, value));
                  }}
                />
              </div>
            </div>
          ))}
          <div className={styles["reset-row"]}>
            <Button
              onClick={() => {
                mapping.update((c) => withDefaults(c, plugin));
              }}
            >
              <RefreshCw size={14} /> Reset to Defaults
            </Button>
          </div>
        </div>
      </div>
    </>
  );
}
