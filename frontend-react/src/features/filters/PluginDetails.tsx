import { RefreshCcw01, Trash01 } from "@untitledui/icons";
import { Badge } from "@/components/base/badges/badges";
import { Button } from "@/components/base/buttons/button";
import { Input } from "@/components/base/input/input";
import { InputNumber } from "@/components/base/input/input-number";
import { Select } from "@/components/base/select/select";
import { Toggle } from "@/components/base/toggle/toggle";
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
import { Section } from "@/ui/Page";

function PropertyControl({
  plugin,
  prop,
  value,
  disabled,
  onChange,
}: {
  plugin: PluginManifest;
  prop: PropertyDescriptor;
  value: PropertyValue | undefined;
  disabled: boolean;
  onChange: (value: PropertyValue) => void;
}) {
  const hint = prop.tooltip ?? undefined;
  const { kind } = prop;
  switch (kind.type) {
    case "Float":
    case "Int": {
      const { config } = kind;
      const current = typeof value === "number" ? value : (config.default ?? 0);
      const unit = config.unit === "" ? "" : ` (${config.unit})`;
      return (
        <InputNumber
          label={`${prop.name}${unit}`}
          hint={hint}
          isDisabled={disabled}
          minValue={config.min ?? undefined}
          maxValue={config.max ?? undefined}
          step={config.step ?? undefined}
          formatOptions={{ maximumFractionDigits: kind.type === "Int" ? 0 : 4, useGrouping: false }}
          value={current}
          onChange={(next) => {
            if (Number.isFinite(next)) {
              onChange(next);
            }
          }}
        />
      );
    }
    case "Bool":
      return (
        <Toggle
          label={prop.name}
          hint={hint}
          isDisabled={disabled}
          isSelected={typeof value === "boolean" ? value : kind.config.default}
          onChange={onChange}
        />
      );
    case "Choice": {
      const { config } = kind;
      return (
        <Select
          label={prop.name}
          hint={hint}
          isDisabled={disabled}
          items={config.options.map((label, id) => ({ id, label }))}
          value={typeof value === "number" ? value : config.default_index}
          onChange={(key) => {
            onChange(Number(key));
          }}
        >
          {(item) => <Select.Item {...item} />}
        </Select>
      );
    }
    case "String":
      return (
        <Input
          label={prop.name}
          hint={hint}
          isDisabled={disabled}
          value={typeof value === "string" ? value : kind.config.default}
          onChange={onChange}
        />
      );
  }
  // A plugin of another version may declare a kind this interface does not know.
  return (
    <p className="text-sm text-tertiary">{`${prop.name}: ${plugin.id} sent an unknown setting.`}</p>
  );
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
    <div className="flex flex-col gap-6">
      <Section
        title={plugin.name}
        description={`By ${plugin.author}`}
        actions={
          <Badge type="pill-color" size="md" color="gray">
            v{plugin.version}
          </Badge>
        }
      >
        <p className="text-md text-secondary">{plugin.description}</p>
        <div className="flex flex-wrap items-center justify-between gap-3">
          <Toggle
            label="Enable this filter"
            isSelected={settings.enabled}
            onChange={(enabled) => {
              mapping.update((c) => withEnabled(c, plugin.id, enabled));
            }}
          />
          <Button color="tertiary-destructive" size="sm" iconLeading={Trash01} onPress={onDelete}>
            Delete this plugin
          </Button>
        </div>
      </Section>

      <Section
        title="Settings"
        description={settings.enabled ? undefined : "Enable the filter to change its settings."}
        actions={
          <Button
            color="secondary"
            size="sm"
            iconLeading={RefreshCcw01}
            onPress={() => {
              mapping.update((c) => withDefaults(c, plugin));
            }}
          >
            Reset to defaults
          </Button>
        }
      >
        {plugin.properties.length === 0 ? (
          <p className="text-sm text-tertiary">This filter has no settings.</p>
        ) : (
          <div className="grid gap-5 md:grid-cols-2">
            {plugin.properties.map((prop) => (
              <PropertyControl
                key={prop.id}
                plugin={plugin}
                prop={prop}
                value={settings.properties[prop.id]}
                disabled={!settings.enabled}
                onChange={(value) => {
                  mapping.update((c) => withProperty(c, plugin.id, prop.id, value));
                }}
              />
            ))}
          </div>
        )}
      </Section>
    </div>
  );
}
