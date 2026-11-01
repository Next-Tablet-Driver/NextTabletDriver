import { Languages } from "lucide-react";
import { useMappingSnapshot, useMappingStore } from "@/features/mapping/MappingProvider";
import { Select } from "@/ui/Select";
import styles from "@/features/settings/LanguageSettings.module.css";

export function LanguageSettings() {
  const { config } = useMappingSnapshot();
  const mapping = useMappingStore();

  return (
    <div className={styles.section}>
      <div className={styles["section-header"]}>
        <Languages size={20} />
        <h2>Language</h2>
      </div>
      {config !== null && (
        <div className={`${styles["flex-column"] ?? ""} ${styles["settings-group"] ?? ""}`}>
          <div className={styles["input-group"]}>
            <label htmlFor="interface_language">Interface Language</label>
            <div className={styles["theme-select-wrapper"]}>
              <Select
                value={config.language}
                options={["English", "Français"]}
                onChange={(language) => {
                  mapping.update((c) => ({ ...c, language }));
                }}
              />
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
