import { useMappingSnapshot, useMappingStore } from "@/features/mapping/MappingProvider";
import { Checkbox } from "@/ui/Checkbox";
import styles from "@/features/pen/HardwareToggles.module.css";

export function HardwareToggles() {
  const { config } = useMappingSnapshot();
  const mapping = useMappingStore();
  return (
    <div className={styles.card}>
      <div className={styles["card-title"]}>Hardware Toggles</div>
      <div className={styles["flex-column"]}>
        {config !== null && (
          <>
            <Checkbox
              label="Disable Pressure"
              checked={config.disable_pressure}
              onChange={(disable_pressure) => {
                mapping.update((c) => ({ ...c, disable_pressure }));
              }}
            />
            <Checkbox
              label="Disable Tilt"
              checked={config.disable_tilt}
              onChange={(disable_tilt) => {
                mapping.update((c) => ({ ...c, disable_tilt }));
              }}
            />
          </>
        )}
      </div>
    </div>
  );
}
