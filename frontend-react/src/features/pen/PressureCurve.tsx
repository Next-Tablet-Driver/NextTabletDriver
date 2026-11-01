import { Activity } from "lucide-react";
import { useMappingSnapshot, useMappingStore } from "@/features/mapping/MappingProvider";
import { Button } from "@/ui/Button";
import styles from "@/features/pen/PressureCurve.module.css";

const TYPES = ["Linear", "Exponential", "Custom"] as const;

export function PressureCurve() {
  const { config } = useMappingSnapshot();
  const mapping = useMappingStore();

  return (
    <div className={`${styles.card ?? ""} ${styles["curve-card"] ?? ""}`}>
      <div className={`${styles["section-header"] ?? ""} ${styles.small ?? ""}`}>
        <Activity size={18} />
        <h3>Pressure Curve</h3>
      </div>
      {config !== null && (
        <div className={styles["segmented-control"]}>
          {TYPES.map((type) => (
            <Button
              key={type}
              variant={config.pressure_curve.curve_type === type ? "primary" : "ghost"}
              onClick={() => {
                mapping.update((c) => ({
                  ...c,
                  pressure_curve: { ...c.pressure_curve, curve_type: type },
                }));
              }}
            >
              {type}
            </Button>
          ))}
        </div>
      )}
      <div className={styles["pressure-visualizer"]}>
        <div className={styles["bar-container"]}>
          <div className={styles["bar-label"]}>0%</div>
          <div className={styles["bar-bg"]}>
            <div className={styles["bar-fill"]} style={{ height: "0%" }} />
          </div>
          <div className={`${styles["bar-label"] ?? ""} ${styles.bottom ?? ""}`}>Raw</div>
        </div>
        <div className={styles["bar-container"]}>
          <div className={styles["bar-label"]}>0%</div>
          <div className={styles["bar-bg"]}>
            <div className={styles["bar-fill"]} style={{ height: "0%" }} />
          </div>
          <div className={`${styles["bar-label"] ?? ""} ${styles.bottom ?? ""}`}>Curve</div>
        </div>
      </div>
    </div>
  );
}
