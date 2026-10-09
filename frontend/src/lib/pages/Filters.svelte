<script lang="ts">
    import { onMount } from "svelte";
    import { Box } from "lucide-svelte";
    import type { MappingState } from "../features/mapping/mapping-state.svelte";
    import { getAvailablePlugins, reloadPlugins, openPluginsFolder, installPlugin, deletePlugin, getUntrustedPlugins, trustPlugin, type UntrustedPlugin, type PluginManifest, type DynamicPluginSettings, type PropertyValue } from "../infrastructure/tauri/commands";
    
    import FiltersSidebar from "../features/filters/FiltersSidebar.svelte";
    import PluginDetails from "../features/filters/PluginDetails.svelte";
    import UntrustedPlugins from "../features/filters/UntrustedPlugins.svelte";
    
    const { mapping } = $props<{ mapping: MappingState }>();
    
    let plugins = $state<PluginManifest[]>([]);
    let activePluginId = $state<string | null>(null);
    let fetchError = $state<string | null>(null);
    let untrusted = $state<UntrustedPlugin[]>([]);

    async function loadPluginsData(): Promise<void> {
        try {
            fetchError = null;
            const res = await getAvailablePlugins();
            plugins = res;
            untrusted = await getUntrustedPlugins();
            if (plugins.length > 0 && activePluginId === null) {
                activePluginId = plugins[0].id;
            } else if (plugins.length > 0 && plugins.find(p => p.id === activePluginId) === undefined) {
                activePluginId = plugins[0].id;
            } else if (plugins.length === 0) {
                activePluginId = null;
            }
        } catch (e: unknown) {
            console.error("Failed to fetch plugins", e);
            fetchError = String(e);
        }
    }

    onMount(() => {
        void loadPluginsData();
    });

    const activePlugin = $derived(plugins.find(p => p.id === activePluginId));
    
    function getPluginSettings(id: string): DynamicPluginSettings {
        if (mapping.config?.plugins?.[id] === undefined) {
            return { enabled: false, properties: {} };
        }
        return mapping.config.plugins[id];
    }
    
    function ensurePluginSettings(id: string): DynamicPluginSettings {
        if (mapping.config === null) throw new Error("Config missing");
        mapping.config.plugins ??= {};
        mapping.config.plugins[id] ??= { enabled: false, properties: {} };
        return mapping.config.plugins[id];
    }
    
    function togglePlugin(id: string): void {
        const settings = ensurePluginSettings(id);
        settings.enabled = !settings.enabled;
        mapping.markDirty();
    }
    
    function setProperty(id: string, propKey: string, value: PropertyValue): void {
        const settings = ensurePluginSettings(id);
        settings.properties[propKey] = value;
        mapping.markDirty();
    }
    
    function getProperty(id: string, propKey: string, defaultValue: PropertyValue): PropertyValue {
        const settings = getPluginSettings(id);
        if (propKey in settings.properties) {
            return settings.properties[propKey];
        }
        return defaultValue;
    }

    function resetToDefaults(id: string): void {
        const plugin = plugins.find(p => p.id === id);
        if (plugin !== undefined) {
            const settings = ensurePluginSettings(id);
            for (const prop of plugin.properties) {
                if (prop.kind.type === 'Float') settings.properties[prop.id] = prop.kind.config.default;
                else if (prop.kind.type === 'Int') settings.properties[prop.id] = prop.kind.config.default;
                else if (prop.kind.type === 'String') settings.properties[prop.id] = prop.kind.config.default;
                else if (prop.kind.type === 'Bool') settings.properties[prop.id] = prop.kind.config.default;
                else settings.properties[prop.id] = (prop.kind as Extract<typeof prop.kind, { type: 'Choice' }>).config.default_index;
            }
            mapping.markDirty();
        }
    }

    async function handleAddPlugin(): Promise<void> {
        try {
            if (await installPlugin()) {
                await loadPluginsData();
            }
        } catch (e) {
            console.error(e);
        }
    }

    async function handleReload(): Promise<void> {
        try {
            plugins = await reloadPlugins();
        } catch (e) {
            console.error(e);
        }
    }

    async function handleOpenFolder(): Promise<void> {
        try {
            await openPluginsFolder();
        } catch (e) {
            console.error(e);
        }
    }

    async function handleTrust(sha256: string): Promise<void> {
        try {
            if (await trustPlugin(sha256)) {
                await loadPluginsData();
            }
        } catch (e) {
            console.error(e);
        }
    }

    async function handleDelete(id: string): Promise<void> {
        try {
            await deletePlugin(id);
            await loadPluginsData();
        } catch (e) {
            console.error(e);
        }
    }
</script>

<div class="filters-container">
    <FiltersSidebar 
        {getPluginSettings} 
        {handleAddPlugin} 
        {handleOpenFolder} 
        {handleReload} 
        {plugins} 
        bind:activePluginId 
    />
    
    <div class="main-panel">
        <UntrustedPlugins onTrust={handleTrust} plugins={untrusted} />
        {#if activePlugin !== undefined}
            <PluginDetails 
                {activePlugin} 
                {getPluginSettings} 
                {getProperty} 
                {handleDelete} 
                {resetToDefaults} 
                {setProperty} 
                {togglePlugin} 
            />
        {:else}
            <div class="empty-state">
                <Box class="empty-icon" size={48} />
                {#if fetchError !== null}
                    <p style:color="red" style:max-width="400px" style:text-align="center">Error fetching plugins: {fetchError}</p>
                {:else}
                    <p>No filters available. Click the + button to install a .dll plugin.</p>
                {/if}
            </div>
        {/if}
    </div>
</div>

<style>
    .filters-container {
        display: flex;
        height: 100%;
        background-color: var(--bg-app);
        color: var(--text-main);
        font-family: var(--font-sans);
    }
    
    .main-panel {
        flex: 1;
        padding: var(--space-12);
        overflow-y: auto;
    }
    
    .empty-state {
        display: flex;
        flex-direction: column;
        align-items: center;
        justify-content: center;
        height: 100%;
        color: var(--text-muted);
        gap: var(--space-4);
    }
</style>
