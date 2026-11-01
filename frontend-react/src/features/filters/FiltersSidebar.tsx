import { Activity, Folder, Plus, RefreshCw, Settings2, WavesHorizontal } from "lucide-react";
import { settingsOf } from "@/features/filters/plugin-settings";
import { useMappingSnapshot } from "@/features/mapping/MappingProvider";
import type { PluginManifest } from "@/infrastructure/tauri/commands";
import { Button } from "@/ui/Button";
import { classes } from "@/ui/classes";
import styles from "@/features/filters/FiltersSidebar.module.css";

/** The icon a filter is drawn with, chosen by what its id says it does. */
export function PluginIcon({ pluginId, size }: { pluginId: string; size: number }) {
  const id = pluginId.toLowerCase();
  if (id.includes("kalman")) {
    return <Settings2 size={size} />;
  }
  if (id.includes("handspeed")) {
    return <WavesHorizontal size={size} />;
  }
  return <Activity size={size} />;
}

export function FiltersSidebar({
  plugins,
  activePluginId,
  onSelect,
  onAdd,
  onOpenFolder,
  onReload,
}: {
  plugins: PluginManifest[];
  activePluginId: string | null;
  onSelect: (id: string) => void;
  onAdd: () => void;
  onOpenFolder: () => void;
  onReload: () => void;
}) {
  const { config } = useMappingSnapshot();
  return (
    <div className={styles.sidebar}>
      <div className={styles["sidebar-header"]}>
        <span>AVAILABLE FILTERS</span>
        <div className={styles["sidebar-actions"]}>
          <Button className="icon-btn" variant="ghost" title="Add Plugin" onClick={onAdd}>
            <Plus size={14} />
          </Button>
          <Button
            className="icon-btn"
            variant="ghost"
            title="Open Plugins Folder"
            onClick={onOpenFolder}
          >
            <Folder size={14} />
          </Button>
          <Button className="icon-btn" variant="ghost" title="Reload Plugins" onClick={onReload}>
            <RefreshCw size={14} />
          </Button>
        </div>
      </div>
      <div className={styles["plugin-list"]}>
        {plugins.map((plugin) => {
          const enabled = config !== null && settingsOf(config, plugin.id).enabled;
          return (
            <button
              key={plugin.id}
              className={classes(
                styles["plugin-item"],
                activePluginId === plugin.id && styles.active,
              )}
              onClick={() => {
                onSelect(plugin.id);
              }}
            >
              <div className={classes(styles["plugin-icon"], enabled && styles.enabled)}>
                <PluginIcon pluginId={plugin.id} size={16} />
              </div>
              <span>{plugin.name}</span>
            </button>
          );
        })}
      </div>
    </div>
  );
}
