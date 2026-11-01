import { useMappingSnapshot, useMappingStore } from "@/features/mapping/MappingProvider";
import { Input } from "@/ui/Input";
import styles from "@/features/pen/Thresholds.module.css";

export function Thresholds() {
  const { config } = useMappingSnapshot();
  const mapping = useMappingStore();
  return (
    <div className={styles.card}>
      <div className={styles["card-title"]}>Thresholds</div>
      <div className={styles["flex-row"]}>
        {config !== null && (
          <>
            <div className={styles["input-group"]}>
              <Input
                label="Tip"
                type="number"
                min={0}
                max={1024}
                width="80px"
                live
                value={config.tip_threshold}
                onCommit={(tip_threshold) => {
                  mapping.update((c) => ({ ...c, tip_threshold }));
                }}
              />
            </div>
            <div className={styles["input-group"]}>
              <Input
                label="Eraser"
                type="number"
                min={0}
                max={1024}
                width="80px"
                live
                value={config.eraser_threshold}
                onCommit={(eraser_threshold) => {
                  mapping.update((c) => ({ ...c, eraser_threshold }));
                }}
              />
            </div>
          </>
        )}
      </div>
    </div>
  );
}
