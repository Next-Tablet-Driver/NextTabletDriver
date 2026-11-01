import { Wifi } from "lucide-react";
import { useMappingSnapshot, useMappingStore } from "@/features/mapping/MappingProvider";
import type { Config } from "@/infrastructure/tauri/commands";
import { Checkbox } from "@/ui/Checkbox";
import { classes } from "@/ui/classes";
import { Input } from "@/ui/Input";
import styles from "@/features/settings/WebSocketSettings.module.css";

const PAYLOAD: {
  key: "send_coordinates" | "send_pressure" | "send_tilt" | "send_status";
  label: string;
}[] = [
  { key: "send_coordinates", label: "Coords" },
  { key: "send_pressure", label: "Pressure" },
  { key: "send_tilt", label: "Tilt" },
  { key: "send_status", label: "Status" },
];

export function WebSocketSettings() {
  const { config } = useMappingSnapshot();
  const mapping = useMappingStore();

  const setSocket = (change: Partial<Config["websocket"]>): void => {
    mapping.update((c) => ({ ...c, websocket: { ...c.websocket, ...change } }));
  };

  return (
    <div className={styles.section}>
      <div className={styles["section-header"]}>
        <Wifi size={20} />
        <h2>WebSocket Server</h2>
      </div>
      {config !== null && (
        <div
          className={classes(styles["flex-column"], styles["settings-group"], styles["ws-group"])}
        >
          <div className={styles["ws-header"]}>
            <Checkbox
              label="Enable WebSocket Server"
              checked={config.websocket.enabled}
              onChange={(enabled) => {
                setSocket({ enabled });
              }}
            />
            <span className={classes(styles.badge, config.websocket.enabled && styles.running)}>
              {config.websocket.enabled ? "RUNNING" : "STOPPED"}
            </span>
          </div>
          <div className={styles["flex-row"]}>
            <div className={styles["input-group"]}>
              <Input
                label="Port"
                type="number"
                width="80px"
                live
                value={config.websocket.port}
                onCommit={(port) => {
                  setSocket({ port });
                }}
              />
            </div>
            <div className={styles["input-group"]}>
              <Input
                label="Rate"
                type="number"
                unit="Hz"
                width="60px"
                live
                value={config.websocket.polling_rate_hz}
                onCommit={(polling_rate_hz) => {
                  setSocket({ polling_rate_hz });
                }}
              />
            </div>
          </div>
          <div className={styles["payload-section"]}>
            <div className={classes(styles.muted, styles["payload-label"])}>Payload Data</div>
            <div className={classes(styles["flex-row"], styles["payload-checkboxes"])}>
              {PAYLOAD.map(({ key, label }) => (
                <Checkbox
                  key={key}
                  label={label}
                  checked={config.websocket[key]}
                  onChange={(value) => {
                    setSocket({ [key]: value });
                  }}
                />
              ))}
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
