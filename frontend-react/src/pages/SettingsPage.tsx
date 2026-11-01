import { Checkbox } from "@/components/base/checkbox/checkbox";
import { InputNumber } from "@/components/base/input/input-number";
import { Select } from "@/components/base/select/select";
import { Toggle } from "@/components/base/toggle/toggle";
import { useMappingSnapshot, useMappingStore } from "@/features/mapping/MappingProvider";
import type { Config } from "@/infrastructure/tauri/commands";
import { THEME_CHOICES, normalizeSelection } from "@/theme/theme";
import { Page, Row, Section } from "@/ui/Page";

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
      <Section title="General">
        <Row label="Run at startup" hint="Start the driver in the background when you log in.">
          <Toggle
            aria-label="Run at startup"
            isSelected={config.run_at_startup}
            onChange={(run_at_startup) => {
              set({ run_at_startup });
            }}
          />
        </Row>
        <Row
          label="Hide to the system tray when minimized"
          hint="Frees the interface while the driver keeps running."
        >
          <Toggle
            aria-label="Hide to the system tray when minimized"
            isSelected={config.system_tray_on_minimize}
            onChange={(system_tray_on_minimize) => {
              set({ system_tray_on_minimize });
            }}
          />
        </Row>
        <Row
          label="Force high resolution timer (0.5 ms)"
          hint="Windows only. Lowers latency and jitter; recommended."
        >
          <Toggle
            aria-label="Force high resolution timer (0.5 ms)"
            isSelected={config.force_high_resolution_timer}
            onChange={(force_high_resolution_timer) => {
              set({ force_high_resolution_timer });
            }}
          />
        </Row>
      </Section>

      <Section title="Appearance">
        <Row label="Theme" hint="System follows the colour scheme of your operating system.">
          <div className="w-40">
            <Select
              aria-label="Theme"
              size="sm"
              items={THEME_CHOICES.map((t) => ({ id: t.value, label: t.label }))}
              value={normalizeSelection(config.theme)}
              onChange={(key) => {
                set({ theme: String(key) });
              }}
            >
              {(item) => <Select.Item {...item} />}
            </Select>
          </div>
        </Row>
        <Row label="Interface language">
          <div className="w-40">
            <Select
              aria-label="Interface language"
              size="sm"
              items={LANGUAGES}
              value={config.language}
              onChange={(key) => {
                set({ language: String(key) });
              }}
            >
              {(item) => <Select.Item {...item} />}
            </Select>
          </div>
        </Row>
      </Section>

      <Section
        title="WebSocket server"
        description="Lets other programs, such as overlays, read the pen in real time."
      >
        <Row
          label="Enable the WebSocket server"
          hint={config.websocket.enabled ? "Running." : "Stopped."}
        >
          <Toggle
            aria-label="Enable the WebSocket server"
            isSelected={config.websocket.enabled}
            onChange={(enabled) => {
              setSocket({ enabled });
            }}
          />
        </Row>
        <Row label="Port">
          <div className="w-32">
            <InputNumber
              aria-label="Port"
              size="sm"
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
        </Row>
        <Row label="Rate" hint="Packets sent per second.">
          <div className="w-32">
            <InputNumber
              aria-label="Rate (Hz)"
              size="sm"
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
        </Row>
        <Row label="Data sent">
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
        </Row>
      </Section>
    </Page>
  );
}
