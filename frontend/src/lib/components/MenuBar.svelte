<script lang="ts">
  interface Props {
    profileName?: string;
  }

  let { profileName = $bindable() }: Props = $props();

  let openMenu = $state<string | null>(null);

  function toggleMenu(menu: string, event: MouseEvent): void {
    event.stopPropagation();
    openMenu = openMenu === menu ? null : menu;
  }

  function closeMenu(): void {
    openMenu = null;
    openSubmenu = null;
    clearTimeout(submenuTimeout);
  }

  import { invoke } from "@tauri-apps/api/core";
  import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
  import { confirm, message } from "@tauri-apps/plugin-dialog";
  import { exportProfile, loadProfile, importOtdProfile } from "../infrastructure/tauri/commands";
  import { os } from "../infrastructure/os";
  import { onMount } from "svelte";
  import DropdownMenu from "./ui/DropdownMenu.svelte";
  import DropdownItem from "./ui/DropdownItem.svelte";
  import DropdownSeparator from "./ui/DropdownSeparator.svelte";

  // Lucide Icons
  import { Folder, Smartphone, Info, FileUp, FileDown, RotateCcw, Save, List, BugPlay, Bug, ChartNoAxesCombined, Link, RefreshCcw} from "lucide-svelte";

  let presets = $state<{name: string, path: string}[]>([]);
  let openSubmenu = $state<string | null>(null);
  let submenuTimeout: number | undefined;

  function handleSubmenuEnter(menu: string): void {
    clearTimeout(submenuTimeout);
    openSubmenu = menu;
  }

  function handleSubmenuLeave(): void {
    submenuTimeout = window.setTimeout(() => {
      openSubmenu = null;
    }, 300);
  }

  onMount(() => {
    void (async (): Promise<void> => {
      try {
        presets = await invoke<{name: string, path: string}[]>("get_presets");
      } catch (e) {
        console.error("Failed to load presets:", e);
      }
    })();
  });
</script>

<svelte:window onclick={closeMenu} />

<div class="menu-bar">
  <div class="menu-container">
    <button
      class="menu-item"
      class:active={openMenu === "file"}
      onclick={(e) => { toggleMenu("file", e); }}
    >
      <Folder size={16}/>
      File
    </button>
    {#if openMenu === "file"}
      <DropdownMenu>
        <DropdownItem onclick={() => {
            void (async (): Promise<void> => {
                closeMenu();
                const path = await os.openFileDialog({
                    multiple: false,
                    filters: [{ name: 'JSON', extensions: ['json'] }]
                });
                if (path !== null && !Array.isArray(path)) {
                    try {
                        await loadProfile(path);
                        window.dispatchEvent(new CustomEvent("reload-config"));
                        profileName = path.split(/[\\/]/).pop() ?? profileName;
                    } catch (err: unknown) {
                        const errMsg = err instanceof Error ? err.message : String(err);
                        await message(`Failed to load settings: ${errMsg}`);
                    }
                }
            })();
        }}><FileDown size={16}/>Load Settings...</DropdownItem>
        <DropdownItem onclick={() => { void invoke("save_config"); closeMenu(); }}><Save size={16}/>Save Settings</DropdownItem>
        <DropdownItem onclick={() => {
            void (async (): Promise<void> => {
                closeMenu();
                const path = await os.saveFileDialog({
                    filters: [{ name: 'JSON', extensions: ['json'] }]
                });
                if (path !== null) {
                    try {
                        await exportProfile(path);
                    } catch (err: unknown) {
                        const errMsg = err instanceof Error ? err.message : String(err);
                        await message(`Failed to save settings as: ${errMsg}`);
                    }
                }
            })();
        }}><Save size={16}/>Save Settings As...</DropdownItem>
        <DropdownItem onclick={() => { 
            void (async (): Promise<void> => {
                try {
                    await invoke("reset_config"); 
                    profileName = "Default Profile";
                    window.dispatchEvent(new CustomEvent("reload-config"));
                } catch (err: unknown) {
                    const errMsg = err instanceof Error ? err.message : String(err);
                    await message(`Failed to reset config: ${errMsg}`);
                }
                closeMenu();
            })();
        }}><RotateCcw size={16}/>Reset to default</DropdownItem>
        <DropdownSeparator />
        <DropdownItem onclick={() => {
            void (async (): Promise<void> => {
                closeMenu();
                const path = await os.saveFileDialog({
                    filters: [{ name: 'JSON', extensions: ['json'] }]
                });
                if (path !== null) {
                    try {
                        await exportProfile(path);
                    } catch (err: unknown) {
                        const errMsg = err instanceof Error ? err.message : String(err);
                        await message(`Failed to export: ${errMsg}`);
                    }
                }
            })();
        }}><FileUp size={16}/>Export .json</DropdownItem>
        <DropdownItem onclick={() => {
            void (async (): Promise<void> => {
                closeMenu();
                const path = await os.openFileDialog({
                    multiple: false,
                    filters: [{ name: 'JSON', extensions: ['json'] }]
                });
                if (path !== null && !Array.isArray(path)) {
                    try {
                        await loadProfile(path);
                        window.dispatchEvent(new CustomEvent("reload-config"));
                        profileName = path.split(/[\\/]/).pop() ?? profileName;
                    } catch (err: unknown) {
                        const errMsg = err instanceof Error ? err.message : String(err);
                        await message(`Failed to import: ${errMsg}`);
                    }
                }
            })();
        }}><FileDown size={16}/>Import .json</DropdownItem>
        <DropdownItem onclick={() => {
            void (async (): Promise<void> => {
                closeMenu();
                const path = await os.openFileDialog({
                    multiple: false,
                    filters: [{ name: 'JSON', extensions: ['json'] }]
                });
                if (path !== null && !Array.isArray(path)) {
                    try {
                        await importOtdProfile(path);
                        window.dispatchEvent(new CustomEvent("reload-config"));
                        profileName = path.split(/[\\/]/).pop() ?? profileName;
                    } catch (err: unknown) {
                        const errMsg = err instanceof Error ? err.message : String(err);
                        await message(`Failed to import OTD settings: ${errMsg}`);
                    }
                }
            })();
        }}><FileDown size={16}/>Import OTD Settings</DropdownItem>
        <DropdownSeparator />
        <DropdownItem hasSubmenu
             onmouseenter={() => { handleSubmenuEnter("presets"); }}
             onmouseleave={handleSubmenuLeave}>
          <List size={16}/>Presets
          {#if openSubmenu === "presets"}
            <DropdownMenu placement="right-start">
                {#if presets.length === 0}
                  <DropdownItem disabled>
                     <span style:font-style="italic">No presets</span>
                  </DropdownItem>
                {:else}
                  {#each presets as preset (preset.path)}
                    <DropdownItem onclick={() => { 
                        void (async (): Promise<void> => {
                            try {
                                await invoke("load_profile", { path: preset.path }); 
                                profileName = preset.name;
                                window.dispatchEvent(new CustomEvent("reload-config"));
                            } catch (err: unknown) {
                                const errMsg = err instanceof Error ? err.message : String(err);
                                await message(`Failed to load preset: ${errMsg}`);
                            }
                            closeMenu();
                        })();
                    }}>{preset.name}</DropdownItem>
                  {/each}
                {/if}
            </DropdownMenu>
          {/if}
        </DropdownItem>
      </DropdownMenu>
    {/if}
  </div>

  <div class="menu-container">
    <button
      class="menu-item"
      class:active={openMenu === "tablet"}
      onclick={(e) => { toggleMenu("tablet", e); }}
    >
      <Smartphone size={16}/>
      Tablet
    </button>
    {#if openMenu === "tablet"}
      <DropdownMenu>
        <!-- TODO (CRITICAL): The detached window spawning via WebviewWindow is currently broken/unstable.
             This MUST be fixed before the first release of this version! -->
        <DropdownItem onclick={() => {
            closeMenu();
            new WebviewWindow('debugger', {
                url: 'index.html?view=debugger',
                title: 'Debugger',
                width: 600,
                height: 500,
                decorations: false,
                transparent: true,
                minWidth: 400,
                minHeight: 300
            });
        }}><BugPlay size={16}/>Open Debugger</DropdownItem>
        <!-- TODO (CRITICAL): The detached window spawning via WebviewWindow is currently broken/unstable.
             This MUST be fixed before the first release of this version! -->
        <DropdownItem onclick={() => {
            closeMenu();
            new WebviewWindow('performance', {
                url: 'index.html?view=performance',
                title: 'Input Lag Analysis',
                width: 500,
                height: 400,
                decorations: false,
                transparent: true,
                minWidth: 400,
                minHeight: 300
            });
        }}><ChartNoAxesCombined size={16}/>Input Lag Analysis</DropdownItem>
      </DropdownMenu>
    {/if}
  </div>

  <div class="menu-container">
    <button
      class="menu-item"
      class:active={openMenu === "help"}
      onclick={(e) => { toggleMenu("help", e); }}
    >
      <Info size={16}/>
      Help
    </button>
    {#if openMenu === "help"}
      <DropdownMenu>
        <DropdownItem onclick={() => { void os.openUrl("https://github.com/Next-Tablet-Driver/NextTabletDriver"); closeMenu(); }}><Link size={16}/>Github Repository</DropdownItem>
        <DropdownItem hasSubmenu
             onmouseenter={() => { handleSubmenuEnter("bug"); }}
             onmouseleave={handleSubmenuLeave}>
          <Bug size={16}/> Report Bug / Suggest Feature
          {#if openSubmenu === "bug"}
            <DropdownMenu placement="right-start">
              <DropdownItem onclick={() => { 
                  void (async (): Promise<void> => {
                      const ver = await invoke("get_core_version");
                      void os.openUrl(`https://github.com/Next-Tablet-Driver/NextTabletDriver/issues/new?template=bug_report.yml&ntd_version=${String(ver)}`); 
                      closeMenu(); 
                  })();
              }}>Report a Bug</DropdownItem>
              <DropdownItem onclick={() => { void os.openUrl("https://github.com/Next-Tablet-Driver/NextTabletDriver/issues/new?template=feature_request.yml"); closeMenu(); }}>Suggest a Feature</DropdownItem>
            </DropdownMenu>
          {/if}
        </DropdownItem>
        <DropdownItem onclick={() => {
            void (async (): Promise<void> => {
                closeMenu();
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
        }}><RefreshCcw size={16} />Check for Update</DropdownItem>
      </DropdownMenu>
    {/if}
  </div>
</div>

<style>
  .menu-bar {
    display: flex;
    padding: var(--space-1);
    gap: var(--space-1);
    background: var(--bg-app);
    flex-shrink: 0;
    border-bottom: var(--border-width) solid var(--border);
  }

  .menu-container {
    position: relative;
  }

  .menu-item {
    display: flex;
    align-items: center;
    gap: var(--space-1-5);
    padding: var(--space-1) var(--space-3);
    font-size: var(--font-md);
    color: var(--text-main);
    cursor: pointer;
    border-radius: var(--radius-md);
    transition: background var(--transition-fast);
    user-select: none;
    background: transparent;
    border: none;
    font-family: inherit;
  }

  .menu-item:hover,
  .menu-item.active {
    background: var(--bg-panel-hover);
  }

</style>
