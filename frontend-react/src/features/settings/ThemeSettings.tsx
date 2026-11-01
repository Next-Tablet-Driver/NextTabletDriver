import { useEffect, useState } from "react";
import {
  Copy,
  Download,
  FolderOpen,
  Globe,
  Palette,
  RefreshCw,
  Trash,
  TriangleAlert,
} from "lucide-react";
import { useMappingSnapshot, useMappingStore } from "@/features/mapping/MappingProvider";
import { os } from "@/infrastructure/os";
import { deleteTheme, importTheme, openThemesFolder } from "@/infrastructure/tauri/commands";
import { useStore } from "@/lib/store";
import {
  BUILTIN_THEMES,
  CUSTOM_PREFIX,
  SYSTEM_THEME,
  buildThemeTemplate,
  documentTokenReader,
  refreshUserThemes,
  themeStore,
} from "@/theme";
import { Button } from "@/ui/Button";
import { classes } from "@/ui/classes";
import { Select } from "@/ui/Select";
import styles from "@/features/settings/ThemeSettings.module.css";

const USER_THEMES_HEADER = "__my-themes";
const THEMES_REPOSITORY = "https://github.com/NextTabletDriver/Themes";

export function ThemeSettings() {
  const { config } = useMappingSnapshot();
  const mapping = useMappingStore();
  const theme = useStore(themeStore.state);
  const [status, setStatus] = useState<string | null>(null);

  useEffect(() => {
    void refreshUserThemes();
  }, []);

  const options = [
    { value: SYSTEM_THEME, label: "System" },
    ...BUILTIN_THEMES.map((t) => ({ value: t.configValue, label: t.label })),
    ...(theme.userThemes.length > 0
      ? [{ value: USER_THEMES_HEADER, label: "- My themes -", disabled: true }]
      : []),
    ...theme.userThemes.map((t) => ({
      value: `${CUSTOM_PREFIX}${t.id}`,
      label: t.definition?.metadata.name ?? t.fileName,
      disabled: t.definition === undefined,
    })),
  ];
  const selectedUserTheme = theme.userThemes.find(
    (t) => `${CUSTOM_PREFIX}${t.id}` === theme.selection,
  );
  const brokenThemes = theme.userThemes.filter((t) => t.definition === undefined);
  const themeNotices = [
    ...(theme.resolved?.fallbackReason === undefined ? [] : [theme.resolved.fallbackReason]),
    ...theme.warnings,
  ];

  const select = (value: string): void => {
    if (config === null || value === USER_THEMES_HEADER) {
      return;
    }
    mapping.update((c) => ({ ...c, theme: value }));
  };

  const handleImport = async (): Promise<void> => {
    try {
      const imported = await importTheme();
      if (imported === null) {
        return;
      }
      await refreshUserThemes();
      const entry = themeStore.userThemes.find((t) => t.id === imported.id);
      if (entry?.definition === undefined) {
        setStatus(`"${imported.file_name}" was imported but cannot be used: see the list below.`);
        return;
      }
      select(`${CUSTOM_PREFIX}${imported.id}`);
      setStatus(`Imported "${entry.definition.metadata.name}".`);
    } catch (e) {
      setStatus(`Import failed: ${String(e)}`);
    }
  };

  const handleDelete = async (): Promise<void> => {
    const entry = selectedUserTheme;
    if (entry === undefined) {
      return;
    }
    const name = entry.definition?.metadata.name ?? entry.fileName;
    if (
      !(await os.confirm(
        `Delete the theme "${name}"? The file will be removed from the themes folder.`,
      ))
    ) {
      return;
    }
    try {
      await deleteTheme(entry.id);
      select("Dark");
      await refreshUserThemes();
      setStatus(`Deleted "${name}".`);
    } catch (e) {
      setStatus(`Delete failed: ${String(e)}`);
    }
  };

  const handleCopyTemplate = async (): Promise<void> => {
    const base = theme.resolved?.render.scheme ?? "dark";
    const template = buildThemeTemplate(documentTokenReader(document.documentElement), base);
    try {
      await navigator.clipboard.writeText(template);
      setStatus("Template copied: paste it into a new .json file in the themes folder.");
    } catch {
      setStatus("Could not access the clipboard.");
    }
  };

  return (
    <div className={styles.section}>
      <div className={styles["section-header"]}>
        <Palette size={20} />
        <h2>Themes</h2>
      </div>
      {config !== null && (
        <div className={classes(styles["flex-column"], styles["settings-group"])}>
          <div className={styles["input-group"]}>
            <label htmlFor="current_theme">Current Theme</label>
            <div className={styles["theme-select-wrapper"]}>
              <Select value={theme.selection} options={options} onChange={select} />
            </div>
          </div>
          <div className={styles["theme-actions"]}>
            <div className={styles["action-buttons"]}>
              <Button onClick={() => void handleImport()}>
                <Download size={16} /> Import theme...
              </Button>
              <Button
                onClick={() => {
                  void refreshUserThemes().then(() => {
                    setStatus("Themes reloaded.");
                  });
                }}
              >
                <RefreshCw size={16} /> Reload
              </Button>
              <Button onClick={() => void openThemesFolder()}>
                <FolderOpen size={16} /> Open Themes Folder
              </Button>
              <Button onClick={() => void handleCopyTemplate()}>
                <Copy size={16} /> Copy as template
              </Button>
              {selectedUserTheme !== undefined && (
                <Button variant="danger-ghost" onClick={() => void handleDelete()}>
                  <Trash size={16} /> Delete
                </Button>
              )}
              <Button onClick={() => void os.openUrl(THEMES_REPOSITORY)}>
                <Globe size={16} /> Browse Online Themes
              </Button>
            </div>
            {status !== null && (
              <p className={styles.status} role="status">
                {status}
              </p>
            )}
          </div>
          {themeNotices.length > 0 && (
            <ul className={styles.notices} aria-label="Theme warnings">
              {themeNotices.map((notice) => (
                <li key={notice}>
                  <TriangleAlert size={14} /> {notice}
                </li>
              ))}
            </ul>
          )}
          {brokenThemes.length > 0 && (
            <div className={styles.broken}>
              <h3>Themes that could not be loaded</h3>
              <ul>
                {brokenThemes.map((broken) => (
                  <li key={broken.id}>
                    <strong>{broken.fileName}</strong>
                    <ul>
                      {broken.errors.map((issue) => (
                        <li key={`${issue.path}:${issue.message}`}>
                          <code>{issue.path === "" ? "file" : issue.path}</code> {issue.message}
                        </li>
                      ))}
                    </ul>
                  </li>
                ))}
              </ul>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
