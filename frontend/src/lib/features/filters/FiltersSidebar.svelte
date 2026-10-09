<script lang="ts">
    import { Plus, Folder, RefreshCw, Activity, Settings2, Waves } from "lucide-svelte";
    import type { PluginManifest, DynamicPluginSettings } from "../../infrastructure/tauri/commands";
    import Button from "../../components/ui/Button.svelte";

    import type { Component } from "svelte";

    let { plugins, activePluginId = $bindable(), handleAddPlugin, handleOpenFolder, handleReload, getPluginSettings } = $props<{
        plugins: PluginManifest[];
        activePluginId: string | null;
        handleAddPlugin: () => void;
        handleOpenFolder: () => void;
        handleReload: () => void;
        getPluginSettings: (id: string) => DynamicPluginSettings;
    }>();

    function getIcon(pluginId: string): Component {
        if (pluginId.toLowerCase().includes("antichatter")) return Activity as unknown as Component;
        if (pluginId.toLowerCase().includes("kalman")) return Settings2 as unknown as Component;
        if (pluginId.toLowerCase().includes("handspeed")) return Waves as unknown as Component;
        return Activity as unknown as Component;
    }
</script>

<div class="sidebar">
    <div class="sidebar-header">
        <span>AVAILABLE FILTERS</span>
        <div class="sidebar-actions">
            <Button class="icon-btn" onclick={handleAddPlugin} title="Add Plugin" variant="ghost"><Plus size={14} /></Button>
            <Button class="icon-btn" onclick={handleOpenFolder} title="Open Plugins Folder" variant="ghost"><Folder size={14} /></Button>
            <Button class="icon-btn" onclick={handleReload} title="Reload Plugins" variant="ghost"><RefreshCw size={14} /></Button>
        </div>
    </div>
    <div class="plugin-list">
        {#each plugins as plugin (plugin.id)}
            {@const Icon = getIcon(plugin.id)}
            <button 
                class="plugin-item" 
                class:active={activePluginId === plugin.id}
                onclick={() => { activePluginId = plugin.id; }}
            >
                <div class="plugin-icon" class:enabled={getPluginSettings(plugin.id).enabled}>
                    <Icon size={16} />
                </div>
                <span>{plugin.name}</span>
            </button>
        {/each}
    </div>
</div>

<style>
    .sidebar {
        width: 260px;
        background-color: var(--bg-panel);
        border-right: var(--border-width) solid var(--border);
        display: flex;
        flex-direction: column;
        padding: var(--space-6) var(--space-3);
        gap: var(--space-4);
    }
    
    .sidebar-header {
        display: flex;
        align-items: center;
        justify-content: space-between;
        padding: 0 var(--space-2);
    }
    
    .sidebar-header span {
        font-size: var(--font-sm);
        font-weight: var(--font-weight-heavy);
        color: var(--text-active);
        text-transform: uppercase;
    }
    
    .sidebar-actions {
        display: flex;
        gap: var(--space-1);
    }
    
    .sidebar-actions :global(.icon-btn) {
        padding: var(--space-1);
        color: var(--text-muted);
        display: flex;
        align-items: center;
        justify-content: center;
    }
    
    .sidebar-actions :global(.icon-btn:hover) {
        color: var(--text-active);
    }
    
    .plugin-list {
        display: flex;
        flex-direction: column;
        gap: var(--space-1);
    }
    
    .plugin-item {
        display: flex;
        align-items: center;
        gap: var(--space-3);
        padding: var(--space-2-5) var(--space-3);
        background: transparent;
        border: none;
        color: var(--text-muted);
        font-size: var(--font-md);
        font-weight: var(--font-weight-bold);
        border-radius: var(--radius-md);
        cursor: pointer;
        text-align: left;
        position: relative;
    }
    
    .plugin-item:hover {
        background: var(--bg-panel-hover);
        color: var(--text-main);
    }
    
    .plugin-item.active {
        background: var(--bg-panel-hover);
        color: var(--text-active);
    }
    
    .plugin-item.active::before {
        content: '';
        position: absolute;
        left: 0;
        top: 20%;
        bottom: 20%;
        width: 3px;
        background-color: var(--accent);
        border-radius: 0 var(--radius-md) var(--radius-md) 0;
    }
    
    .plugin-icon {
        display: flex;
        opacity: 0.5;
    }
    
    .plugin-icon.enabled {
        color: var(--accent);
        opacity: 1;
    }
</style>
