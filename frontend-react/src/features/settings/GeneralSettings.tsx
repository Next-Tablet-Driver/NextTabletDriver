import { useState } from "react";
import { Settings as SettingsIcon } from "lucide-react";
import { useMappingSnapshot, useMappingStore } from "@/features/mapping/MappingProvider";
import type { Config } from "@/infrastructure/tauri/commands";
import { Checkbox } from "@/ui/Checkbox";
import styles from "@/features/settings/GeneralSettings.module.css";

export function GeneralSettings() {
  const { config } = useMappingSnapshot();
  const mapping = useMappingStore();
  // Not connected to a setting yet: it only reflects the choice on screen, as before.
  const [statistics, setStatistics] = useState(true);

  const set = (change: Partial<Config>): void => {
    mapping.update((c) => ({ ...c, ...change }));
  };

  return (
    <div className={styles.section}>
      <div className={styles["section-header"]}>
        <SettingsIcon size={20} />
        <h2>General Settings</h2>
      </div>
      {config !== null && (
        <div className={`${styles["flex-column"] ?? ""} ${styles["settings-group"] ?? ""}`}>
          <Checkbox
            label="Run at startup"
            tooltip="Automatically start the driver in the background on system boot."
            checked={config.run_at_startup}
            onChange={(run_at_startup) => {
              set({ run_at_startup });
            }}
          />
          <Checkbox
            label="System Tray when Minimize"
            tooltip="Hide to tray instead of taskbar. Unloads the UI to maximize performance."
            checked={config.system_tray_on_minimize}
            onChange={(system_tray_on_minimize) => {
              set({ system_tray_on_minimize });
            }}
          />
          <Checkbox
            label="Force High Resolution Timer (0.5ms)"
            tooltip="Forces the Windows system timer to 0.5ms to reduce latency and jitter (Recommended)."
            checked={config.force_high_resolution_timer}
            onChange={(force_high_resolution_timer) => {
              set({ force_high_resolution_timer });
            }}
          />
          <Checkbox
            label="Enable Anonymous Usage Statistics"
            tooltip="Share anonymous hardware and crash data to help improve the driver."
            checked={statistics}
            onChange={setStatistics}
          />
        </div>
      )}
    </div>
  );
}
