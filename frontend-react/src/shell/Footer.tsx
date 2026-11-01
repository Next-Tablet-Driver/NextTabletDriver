import { useAppStatus } from "@/app/app-status";
import { useMappingSnapshot, useMappingStore } from "@/features/mapping/MappingProvider";
import type { DriverMode } from "@/infrastructure/tauri/commands";
import { Select } from "@/ui/Select";
import { classes } from "@/ui/classes";
import styles from "@/shell/Footer.module.css";

const MODES = [
  { value: "Absolute", label: "Absolute Mode" },
  { value: "Relative", label: "Relative Mode" },
];

/** The driver mode, the tablet, the profile (starred when unsaved) and the version. */
export function Footer() {
  const { version, profileName, tablet } = useAppStatus();
  const { config, isDirty } = useMappingSnapshot();
  const mapping = useMappingStore();

  return (
    <div className={styles.footer}>
      <div className={styles.left}>
        <Select
          value={config?.mode ?? "Absolute"}
          options={MODES}
          onChange={(mode) => {
            mapping.update((c) => ({ ...c, mode: mode as DriverMode }));
          }}
        />
      </div>
      <div className={styles.right}>
        <Select value={tablet.name} options={[{ value: tablet.name, label: tablet.name }]} />
        <span className={classes(styles.profile, isDirty && styles.dirty)}>
          Profile:{" "}
          <strong
            style={{
              color: "var(--text-emphasis)",
              fontStyle: isDirty ? "italic" : "normal",
            }}
          >
            {profileName}
            {isDirty ? "*" : ""}
          </strong>
        </span>
        <span className={styles.version}>{version}</span>
      </div>
    </div>
  );
}
