import { useMappingSnapshot } from "@/features/mapping/MappingProvider";
import { GeneralSettings } from "@/features/settings/GeneralSettings";
import { LanguageSettings } from "@/features/settings/LanguageSettings";
import { ThemeSettings } from "@/features/settings/ThemeSettings";
import { WebSocketSettings } from "@/features/settings/WebSocketSettings";
import styles from "@/pages/SettingsPage.module.css";

export function SettingsPage() {
  const { config } = useMappingSnapshot();
  return (
    <div className={styles["page-container"]}>
      {config !== null && (
        <>
          <GeneralSettings />
          <ThemeSettings />
          <LanguageSettings />
          <WebSocketSettings />
        </>
      )}
    </div>
  );
}
