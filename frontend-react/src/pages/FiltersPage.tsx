import { useState } from "react";
import { Box } from "lucide-react";
import { FiltersSidebar } from "@/features/filters/FiltersSidebar";
import { PluginDetails } from "@/features/filters/PluginDetails";
import { UntrustedPlugins } from "@/features/filters/UntrustedPlugins";
import { useCommand } from "@/hooks/useCommand";
import { log } from "@/infrastructure/log";
import { os } from "@/infrastructure/os";
import {
  deletePlugin,
  getAvailablePlugins,
  getUntrustedPlugins,
  installPlugin,
  openPluginsFolder,
  reloadPlugins,
  trustPlugin,
  type PluginManifest,
  type UntrustedPlugin,
} from "@/infrastructure/tauri/commands";
import styles from "@/pages/FiltersPage.module.css";

async function loadPlugins(): Promise<{ plugins: PluginManifest[]; untrusted: UntrustedPlugin[] }> {
  const [plugins, untrusted] = await Promise.all([getAvailablePlugins(), getUntrustedPlugins()]);
  return { plugins, untrusted };
}

/** The filters (plugins) that run between the tablet and the cursor. */
export function FiltersPage() {
  const { state, reload } = useCommand(loadPlugins);
  const [chosen, setChosen] = useState<string | null>(null);

  const plugins = state.status === "ready" ? state.data.plugins : [];
  const untrusted = state.status === "ready" ? state.data.untrusted : [];
  const fetchError = state.status === "error" ? state.message : null;
  // Keep the plugin on screen if it is still there, else show the first one.
  const activeId = plugins.some((p) => p.id === chosen) ? chosen : (plugins[0]?.id ?? null);
  const active = plugins.find((p) => p.id === activeId);

  /** Runs a plugin-folder action, then reads the list again; a failure is logged. */
  const run = (action: () => Promise<unknown>) => (): void => {
    action().then(reload, (e: unknown) => {
      log.error(e);
    });
  };

  return (
    <div className={styles["filters-container"]}>
      <FiltersSidebar
        plugins={plugins}
        activePluginId={activeId}
        onSelect={setChosen}
        onAdd={run(installPlugin)}
        onOpenFolder={run(openPluginsFolder)}
        onReload={run(reloadPlugins)}
      />
      <div className={styles["main-panel"]}>
        <UntrustedPlugins
          plugins={untrusted}
          onTrust={(sha256) => {
            run(() => trustPlugin(sha256))();
          }}
        />
        {active !== undefined ? (
          <PluginDetails
            key={active.id}
            plugin={active}
            onDelete={() => {
              void os
                .confirm(`Delete the plugin "${active.name}" from your computer?`)
                .then((yes) => {
                  if (yes) {
                    run(() => deletePlugin(active.id))();
                  }
                });
            }}
          />
        ) : (
          <div className={styles["empty-state"]}>
            <Box className={styles["empty-icon"]} size={48} />
            {fetchError !== null ? (
              <p style={{ color: "red", maxWidth: "400px", textAlign: "center" }}>
                Error fetching plugins: {fetchError}
              </p>
            ) : (
              <p>No filters available. Click the + button to install a .dll plugin.</p>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
