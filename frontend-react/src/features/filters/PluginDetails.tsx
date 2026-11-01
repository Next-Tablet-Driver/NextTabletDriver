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
import { Row, Section } from "@/ui/Page";

function PropertyRow({
  prop,
  value,
  disabled,
  onChange,
}: {
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
      const unit = config.unit === "" ? "" : ` (${config.unit})`;
      return (
        <Row label={`${prop.name}${unit}`} hint={hint}>
          <div className="w-32">
            <InputNumber
              aria-label={`${prop.name}${unit}`}
              size="sm"
              isDisabled={disabled}
              minValue={config.min ?? undefined}
              maxValue={config.max ?? undefined}
              step={config.step ?? undefined}
              formatOptions={{
                maximumFractionDigits: kind.type === "Int" ? 0 : 4,
                useGrouping: false,
              }}
              value={typeof value === "number" ? value : (config.default ?? 0)}
              onChange={(next) => {
                if (Number.isFinite(next)) {
                  onChange(next);
                }
              }}
            />
          </div>
        </Row>
      );
    }
    case "Bool":
      return (
        <Row label={prop.name} hint={hint}>
          <Toggle
            aria-label={prop.name}
            isDisabled={disabled}
            isSelected={typeof value === "boolean" ? value : kind.config.default}
            onChange={onChange}
          />
        </Row>
      );
    case "Choice": {
      const { config } = kind;
      return (
        <Row label={prop.name} hint={hint}>
          <div className="w-44">
            <Select
              aria-label={prop.name}
              size="sm"
              isDisabled={disabled}
              items={config.options.map((label, id) => ({ id, label }))}
              value={typeof value === "number" ? value : config.default_index}
              onChange={(key) => {
                onChange(Number(key));
              }}
            >
              {(item) => <Select.Item {...item} />}
            </Select>
          </div>
        </Row>
      );
    }
    case "String":
      return (
        <Row label={prop.name} hint={hint}>
          <div className="w-44">
            <Input
              aria-label={prop.name}
              size="sm"
              isDisabled={disabled}
              value={typeof value === "string" ? value : kind.config.default}
              onChange={onChange}
            />
          </div>
        </Row>
      );
  }
  // A plugin of another version may declare a kind this interface does not know.
  return <Row label={prop.name} hint="This filter sent a setting this version does not know." />;
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
      <Section
        title={plugin.name}
        description={`Version ${plugin.version}, by ${plugin.author}`}
        actions={
          <Button color="tertiary-destructive" size="sm" onPress={onDelete}>
            Delete this plugin
          </Button>
        }
      >
        <p className="pb-2 text-sm text-secondary">{plugin.description}</p>
        <Row label="Enable this filter">
          <Toggle
            aria-label="Enable this filter"
            isSelected={settings.enabled}
            onChange={(enabled) => {
              mapping.update((c) => withEnabled(c, plugin.id, enabled));
            }}
          />
        </Row>
      </Section>

      <Section
        title="Settings"
        description={settings.enabled ? undefined : "Enable the filter to change its settings."}
        actions={
          <Button
            color="secondary"
            size="sm"
            onPress={() => {
              mapping.update((c) => withDefaults(c, plugin));
            }}
          >
            Reset to defaults
          </Button>
        }
      >
        {plugin.properties.length === 0 ? (
          <p className="py-2 text-sm text-tertiary">This filter has no settings.</p>
        ) : (
          plugin.properties.map((prop) => (
            <PropertyRow
              key={prop.id}
              prop={prop}
              value={settings.properties[prop.id]}
              disabled={!settings.enabled}
              onChange={(value) => {
                mapping.update((c) => withProperty(c, plugin.id, prop.id, value));
              }}
            />
          ))
        )}
      </Section>
    </>
  );
}
