import { Badge } from "@/components/base/badges/badges";
import { Checkbox } from "@/components/base/checkbox/checkbox";
import { InputNumber } from "@/components/base/input/input-number";
import { Select } from "@/components/base/select/select";
import { Toggle } from "@/components/base/toggle/toggle";
import { useMappingSnapshot, useMappingStore } from "@/features/mapping/MappingProvider";
import type { Config } from "@/infrastructure/tauri/commands";
import { THEME_CHOICES, normalizeSelection } from "@/theme/theme";
import { Page, Section } from "@/ui/Page";

const LANGUAGES = [
  { id: "English", label: "English" },
  { id: "Français", label: "Français" },
];

const PAYLOAD: { key: keyof Config["websocket"]; label: string }[] = [
  { key: "send_coordinates", label: "Coordinates" },
  { key: "send_pressure", label: "Pressure" },
  { key: "send_tilt", label: "Tilt" },
  { key: "send_status", label: "Status" },
];

/** General behaviour, appearance, language and the WebSocket server. */
export function SettingsPage() {
  const { config } = useMappingSnapshot();
  const mapping = useMappingStore();

  if (config === null) {
    return null;
  }

  const set = (change: Partial<Config>): void => {
    mapping.update((c) => ({ ...c, ...change }));
  };
  const setSocket = (change: Partial<Config["websocket"]>): void => {
    mapping.update((c) => ({ ...c, websocket: { ...c.websocket, ...change } }));
  };

  return (
    <Page
      title="Settings"
      description="How the driver starts, how it looks, and what it shares with other programs."
    >
      <Section title="General" description="Start-up and timing.">
        <Toggle
          label="Run at startup"
          hint="Start the driver in the background when you log in."
          isSelected={config.run_at_startup}
          onChange={(run_at_startup) => {
            set({ run_at_startup });
          }}
        />
        <Toggle
          label="Hide to the system tray when minimized"
          hint="Frees the interface while the driver keeps running."
          isSelected={config.system_tray_on_minimize}
          onChange={(system_tray_on_minimize) => {
            set({ system_tray_on_minimize });
          }}
        />
        <Toggle
          label="Force high resolution timer (0.5 ms)"
          hint="Windows only. Lowers latency and jitter; recommended."
          isSelected={config.force_high_resolution_timer}
          onChange={(force_high_resolution_timer) => {
            set({ force_high_resolution_timer });
          }}
        />
      </Section>

      <Section title="Appearance" description="The colour scheme of the interface.">
        <div className="max-w-xs">
          <Select
            label="Theme"
            items={THEME_CHOICES.map((t) => ({ id: t.value, label: t.label }))}
            value={normalizeSelection(config.theme)}
            onChange={(key) => {
              set({ theme: String(key) });
            }}
          >
            {(item) => <Select.Item {...item} />}
          </Select>
        </div>
      </Section>

      <Section title="Language">
        <div className="max-w-xs">
          <Select
            label="Interface language"
            items={LANGUAGES}
            value={config.language}
            onChange={(key) => {
              set({ language: String(key) });
            }}
          >
            {(item) => <Select.Item {...item} />}
          </Select>
        </div>
      </Section>

      <Section
        title="WebSocket server"
        description="Lets other programs, such as overlays, read the pen in real time."
        actions={
          <Badge type="pill-color" size="md" color={config.websocket.enabled ? "success" : "gray"}>
            {config.websocket.enabled ? "Running" : "Stopped"}
          </Badge>
        }
      >
        <Toggle
          label="Enable the WebSocket server"
          isSelected={config.websocket.enabled}
          onChange={(enabled) => {
            setSocket({ enabled });
          }}
        />
        <div className="flex flex-wrap gap-4">
          <div className="w-40">
            <InputNumber
              label="Port"
              minValue={1}
              maxValue={65535}
              formatOptions={{ useGrouping: false }}
              value={config.websocket.port}
              onChange={(port) => {
                if (Number.isFinite(port)) {
                  setSocket({ port });
                }
              }}
            />
          </div>
          <div className="w-40">
            <InputNumber
              label="Rate (Hz)"
              minValue={1}
              maxValue={1000}
              value={config.websocket.polling_rate_hz}
              onChange={(polling_rate_hz) => {
                if (Number.isFinite(polling_rate_hz)) {
                  setSocket({ polling_rate_hz });
                }
              }}
            />
          </div>
        </div>
        <fieldset className="flex flex-col gap-3">
          <legend className="mb-1 text-sm font-medium text-secondary">Data sent</legend>
          <div className="flex flex-wrap gap-x-6 gap-y-3">
            {PAYLOAD.map(({ key, label }) => (
              <Checkbox
                key={key}
                label={label}
                isSelected={config.websocket[key] as boolean}
                onChange={(value) => {
                  setSocket({ [key]: value });
                }}
              />
            ))}
          </div>
        </fieldset>
      </Section>
    </Page>
  );
}
