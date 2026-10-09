<script lang="ts">
  import { windowControls } from "../infrastructure/window";
  import { check, type Update } from "@tauri-apps/plugin-updater";
  import { os } from "../infrastructure/os";
  import { onMount } from "svelte";
  import { Minus, RefreshCcw, Square, X } from "lucide-svelte";
  import iconUrl from "../../assets/icon.png";
  import Button from "./ui/Button.svelte";

  const { title = "NextTabletDriver", systemTrayOnMinimize = false } = $props<{ title?: string, systemTrayOnMinimize?: boolean }>();

  function minimize(): void {
    if (systemTrayOnMinimize) {
      void windowControls.hide();
    } else {
      void windowControls.minimize();
    }
  }

  function toggleMaximize(): void {
    void windowControls.toggleMaximize();
  }

  function close(): void {
    void windowControls.close();
  }

  let updateAvailable = $state(false);
  let updateObj: Update | null = $state(null);
  let isUpdating = $state(false);
  let updateProgress = $state(0);

  onMount(() => {
    void (async (): Promise<void> => {
      try {
        const u = await check();
        if (u !== null) {
          updateObj = u;
          updateAvailable = true;
        }
      } catch (e) {
        console.error("Failed to check for updates:", e);
      }
    })();
  });

  async function installUpdate(): Promise<void> {
    if (updateObj === null || isUpdating) return;
    try {
      isUpdating = true;
      let downloaded = 0;
      let contentLength = 0;

      await updateObj.downloadAndInstall((event) => {
        switch (event.event) {
          case 'Started':
            contentLength = event.data.contentLength ?? 0;
            break;
          case 'Progress':
            downloaded += event.data.chunkLength;
            if (contentLength > 0) {
              updateProgress = Math.round((downloaded / contentLength) * 100);
            }
            break;
          case 'Finished':
            updateProgress = 100;
            break;
        }
      });
      await os.relaunch();
    } catch (e) {
      console.error("Update installation failed:", e);
      isUpdating = false;
    }
  }
</script>

<div class="titlebar" data-tauri-drag-region>
  <div class="icon-container" data-tauri-drag-region>
    <img class="app-icon" alt="App Icon" src={iconUrl} />
  </div>

  <div class="title" data-tauri-drag-region>
    {title}
  </div>

  {#if updateAvailable}
    <Button class="update-btn" disabled={isUpdating} onclick={installUpdate} variant="primary">
      <RefreshCcw class={isUpdating ? "spinning" : ""} size={13} strokeWidth={2.5} />
      <span>{isUpdating ? `Updating (${String(updateProgress)}%)` : 'Update'}</span>
    </Button>
  {/if}

  <div class="window-controls">
    <button class="control" onclick={minimize}>
      <Minus size={16} strokeWidth={1.8} />
    </button>
    <button class="control" onclick={toggleMaximize}>
      <Square size={12} strokeWidth={2} />
    </button>
    <button class="control close" onclick={close}>
      <X size={18} strokeWidth={1.8} />
    </button>
  </div>
</div>

<style>
  .titlebar {
    height: 32px;
    background: var(--bg-app);
    display: flex;
    align-items: center;
    justify-content: space-between;
    user-select: none;
    flex-shrink: 0;
    border-bottom: var(--border-width) solid var(--overlay-20);
  }

  .icon-container {
    width: 32px;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    pointer-events: none;
  }

  .app-icon {
    width: 18px;
    height: 18px;
    object-fit: contain;
    aspect-ratio: 1/1;
  }

  .title {
    font-size: var(--font-md);
    color: var(--text-muted);
    flex-grow: 1;
    text-align: left;
    padding-left: var(--space-0-5);
    pointer-events: none;
  }

  :global(.update-btn) {
    margin-right: var(--space-2);
    padding: var(--space-0-5) var(--space-3);
    font-size: var(--font-md);
    font-weight: var(--font-weight-bold);
  }

  .window-controls {
    display: flex;
    height: 100%;
  }

  .control {
    width: 46px;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-muted);
    cursor: pointer;
    transition: background var(--transition-fast);
    background: transparent;
    border: none;
    padding: 0;
  }

  .control:hover {
    background: var(--bg-panel);
    color: var(--text-main);
  }

  .control.close:hover {
    background: var(--titlebar-close-bg);
    color: var(--titlebar-close-fg);
  }

  :global(.spinning) {
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    100% {
      transform: rotate(360deg);
    }
  }
</style>
