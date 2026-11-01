import { useState } from "react";
import { FolderPlus, Plus, RefreshCcw01 } from "@untitledui/icons";
import { Button } from "@/components/base/buttons/button";
import { Tab, TabList, TabPanel, Tabs } from "@/components/application/tabs/tabs";
import { EmptyState } from "@/components/application/empty-state/empty-state";
import { PluginDetails } from "@/features/filters/PluginDetails";
import { UntrustedPlugins } from "@/features/filters/UntrustedPlugins";
import { settingsOf } from "@/features/filters/plugin-settings";
import { useCommand } from "@/hooks/useCommand";
import { useMappingSnapshot } from "@/features/mapping/MappingProvider";
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
import { Page } from "@/ui/Page";

async function loadPlugins(): Promise<{ plugins: PluginManifest[]; untrusted: UntrustedPlugin[] }> {
  const [plugins, untrusted] = await Promise.all([getAvailablePlugins(), getUntrustedPlugins()]);
  return { plugins, untrusted };
}

/** The filters (plugins) that run between the tablet and the cursor. */
export function FiltersPage() {
  const { config } = useMappingSnapshot();
  const { state, reload } = useCommand(loadPlugins);
  const [chosen, setChosen] = useState<string | null>(null);

  const plugins = state.status === "ready" ? state.data.plugins : [];
  const untrusted = state.status === "ready" ? state.data.untrusted : [];
  const error = state.status === "error" ? state.message : null;
  // Keep the plugin on screen if it is still there, else show the first one.
  const active = plugins.some((p) => p.id === chosen) ? chosen : (plugins[0]?.id ?? null);

  /** Runs a plugin-folder action, then reads the list again; a failure is logged. */
  const run = (action: () => Promise<unknown>) => (): void => {
    action().then(reload, (e: unknown) => {
      log.error(e);
    });
  };

  const current = plugins.find((p) => p.id === active);

  return (
    <Page
      title="Filters"
      description="Plugins that smooth or adjust the pen before it moves the cursor."
      actions={
        <div className="flex flex-wrap gap-2">
          <Button color="secondary" size="sm" iconLeading={Plus} onPress={run(installPlugin)}>
            Add a plugin
          </Button>
          <Button
            color="secondary"
            size="sm"
            iconLeading={FolderPlus}
            onPress={run(openPluginsFolder)}
          >
            Open the folder
          </Button>
          <Button
            color="secondary"
            size="sm"
            iconLeading={RefreshCcw01}
            onPress={run(reloadPlugins)}
          >
            Reload
          </Button>
        </div>
      }
    >
      <UntrustedPlugins
        plugins={untrusted}
        onTrust={(sha256) => {
          run(() => trustPlugin(sha256))();
        }}
      />

      {error !== null && (
        <p role="alert" className="text-sm text-error-primary">
          Could not read the plugins: {error}
        </p>
      )}

      {plugins.length === 0 ? (
        error === null && (
          <EmptyState size="md">
            <EmptyState.Header>
              <EmptyState.FeaturedIcon color="gray" />
            </EmptyState.Header>
            <EmptyState.Content>
              <EmptyState.Title>No filter installed</EmptyState.Title>
              <EmptyState.Description>
                Add a plugin, or put its library in the plugins folder and reload.
              </EmptyState.Description>
            </EmptyState.Content>
          </EmptyState>
        )
      ) : (
        <Tabs
          orientation="vertical"
          selectedKey={active ?? undefined}
          onSelectionChange={(key) => {
            setChosen(String(key));
          }}
          className="flex flex-col gap-6 md:flex-row md:items-start"
        >
          <TabList
            aria-label="Filters"
            type="button-gray"
            orientation="vertical"
            className="max-md:w-full md:w-56 md:shrink-0"
          >
            {plugins.map((plugin) => (
              <Tab
                key={plugin.id}
                id={plugin.id}
                label={plugin.name}
                badge={config !== null && settingsOf(config, plugin.id).enabled ? "On" : undefined}
              />
            ))}
          </TabList>
          <div className="min-w-0 flex-1">
            {current !== undefined && (
              <TabPanel id={current.id}>
                <PluginDetails
                  key={current.id}
                  plugin={current}
                  onDelete={() => {
                    void os
                      .confirm(`Delete the plugin "${current.name}" from your computer?`)
                      .then((yes) => {
                        if (yes) {
                          run(() => deletePlugin(current.id))();
                        }
                      });
                  }}
                />
              </TabPanel>
            )}
          </div>
        </Tabs>
      )}
    </Page>
  );
}
