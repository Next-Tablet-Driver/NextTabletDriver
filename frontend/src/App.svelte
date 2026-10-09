<script lang="ts">
  import { onMount, type Component } from "svelte";
  import { refreshUserThemes, themeStore } from "./lib/theme";
  import { confirm, message } from "@tauri-apps/plugin-dialog";
  import TitleBar from "./lib/components/TitleBar.svelte";
  import MenuBar from "./lib/components/MenuBar.svelte";
  import TabBar from "./lib/components/TabBar.svelte";
  import Footer from "./lib/components/Footer.svelte";
  import Output from "./lib/pages/Output.svelte";
  import Console from "./lib/pages/Console.svelte";
  import Release from "./lib/pages/Release.svelte";
  import Credits from "./lib/pages/Credits.svelte";
  import Filters from "./lib/pages/Filters.svelte";
  import PenSettings from "./lib/pages/PenSettings.svelte";
  import Settings from "./lib/pages/Settings.svelte";
  import Debugger from "./lib/pages/Debugger.svelte";
  import Performance from "./lib/pages/Performance.svelte";
  import { getCoreVersion, getTabletStatus, getCurrentProfileName, type TabletStatus } from "./lib/infrastructure/tauri/commands";
  import { events } from "./lib/infrastructure/events";
  import { os } from "./lib/infrastructure/os";
  import { MappingState } from "./lib/features/mapping/mapping-state.svelte";

  let version = $state("Loading...");
  let activeTab = $state("output");
  
  const tabComponents = {
      output: Output,
      console: Console,
      release: Release,
      credits: Credits,
      filters: Filters,
      penSettings: PenSettings,
      settings: Settings
  };

  const ActiveTab = $derived(
      tabComponents[activeTab as keyof typeof tabComponents] as Component<{ mapping: MappingState }> | undefined
  );
  
  // Read query parameters to determine if we are a detached window
  const urlParams = new URLSearchParams(window.location.search);
  const currentView = urlParams.get("view");
  
  let tabletStatus = $state<TabletStatus>({
      connected: false,
      name: "No Tablet Detected",
      width: 0,
      height: 0
  });
  
  const mapping = new MappingState();
  let profileName = $state("Default Profile");

  onMount(() => {
      let unlistenDevice: (() => void) | undefined;
      let unlistenProfile: (() => void) | undefined;
      let unlistenUpdates: (() => void) | undefined;
      const reloadConfigHandler = (): void => {
          void mapping.load();
      };
      
      void (async (): Promise<void> => {
          try {
              version = await getCoreVersion();
              tabletStatus = await getTabletStatus();
              profileName = await getCurrentProfileName();
              await mapping.load();
              await refreshUserThemes();
              
              window.addEventListener("reload-config", reloadConfigHandler);
              
              unlistenDevice = await events.onDeviceChanged(() => {
                  void getTabletStatus().then(status => {
                      tabletStatus = status;
                  });
              });

              unlistenProfile = await events.onProfileLoaded(() => {
                  void (async (): Promise<void> => {
                      profileName = await getCurrentProfileName();
                      await mapping.load();
                  })();
              });

              unlistenUpdates = await events.onNavigateToUpdates(() => {
                  void (async (): Promise<void> => {
                      try {
                          const { check } = await import('@tauri-apps/plugin-updater');
                          const update = await check();
                          if (update !== null) {
                              const yes = await confirm(`Update v${update.version} is available! Do you want to download and install it now?`);
                              if (yes) {
                                  await update.downloadAndInstall();
                                  await os.relaunch();
                              }
                          } else {
                              await message("You are already on the latest version.");
                          }
                      } catch (err) {
                          await message(`Failed to check for updates: ${String(err)}`);
                      }
                  })();
              });
          } catch (e) {
              console.error("Failed to call Tauri commands:", e);
          }
      })();
      
      return () => {
          window.removeEventListener("reload-config", reloadConfigHandler);
          if (unlistenDevice !== undefined) unlistenDevice();
          if (unlistenProfile !== undefined) unlistenProfile();
          if (unlistenUpdates !== undefined) unlistenUpdates();
      };
  });

  $effect(() => {
      // The configuration is the source of truth: re-apply the theme whenever it changes.
      const selected = mapping.config?.theme;
      if (selected !== undefined && selected !== "") themeStore.select(selected);
  });

  onMount(() => {
      themeStore.start();
      return () => { themeStore.stop(); };
  });
</script>

{#if import.meta.env.DEV && currentView === "design-system"}
    {#await import("./dev/DesignSystem.svelte") then { default: DesignSystem }}
        <DesignSystem />
    {/await}
{:else if currentView === "debugger"}
    <Debugger />
{:else if currentView === "performance"}
    <Performance />
{:else}
<main id="app">
  <TitleBar 
    systemTrayOnMinimize={mapping.config?.system_tray_on_minimize ?? false} 
    title={`NextTabletDriver ${  version}`} 
  />
  <MenuBar bind:profileName />
  <TabBar bind:activeTab />
  
  <div class="content-area">
      {#if ActiveTab}
          <ActiveTab {mapping} />
      {:else}
          <div class="placeholder">
              <h2>{activeTab}</h2>
              <p>This tab is under construction during the UI migration.</p>
          </div>
      {/if}
  </div>
  
  <Footer 
      isDirty={mapping.isDirty}
      {profileName}
      tabletName={tabletStatus.name}
      {version}
      bind:activeMode={mapping.driverMode}
  />
</main>
{/if}

<style>
  .content-area {
      flex-grow: 1;
      background-color: var(--bg-app);
      position: relative;
      overflow: hidden;
  }
  
  .placeholder {
      display: flex;
      flex-direction: column;
      align-items: center;
      justify-content: center;
      height: 100%;
      color: var(--text-muted);
  }
</style>
