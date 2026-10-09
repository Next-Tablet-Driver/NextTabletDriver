<script lang="ts">
    import { Activity, Settings2, Waves, Trash2, Info, RefreshCw } from "lucide-svelte";
    import type { PluginManifest, DynamicPluginSettings, PropertyValue } from "../../infrastructure/tauri/commands";
    import Checkbox from "../../components/ui/Checkbox.svelte";
    import Button from "../../components/ui/Button.svelte";
    import Input from "../../components/ui/Input.svelte";
    import Select from "../../components/ui/Select.svelte";

    const { 
        activePlugin, 
        getPluginSettings, 
        togglePlugin, 
        handleDelete, 
        setProperty, 
        getProperty, 
        resetToDefaults 
    } = $props<{
        activePlugin: PluginManifest;
        getPluginSettings: (id: string) => DynamicPluginSettings;
        togglePlugin: (id: string) => void;
        handleDelete: (id: string) => void;
        setProperty: (id: string, propKey: string, value: PropertyValue) => void;
        getProperty: (id: string, propKey: string, defaultValue: PropertyValue) => PropertyValue;
        resetToDefaults: (id: string) => void;
    }>();

    function getIcon(pluginId: string): typeof Activity {
        if (pluginId.toLowerCase().includes("antichatter")) return Activity;
        if (pluginId.toLowerCase().includes("kalman")) return Settings2;
        if (pluginId.toLowerCase().includes("handspeed")) return Waves;
        return Activity;
    }

    const Icon = $derived(getIcon(activePlugin.id));
</script>

<div class="plugin-header-area">
    <div class="plugin-title-row">
        <span class="header-icon"><Icon size={24} /></span>
        <h2>{activePlugin.name}</h2>
        <span class="badge">v{activePlugin.version}</span>
    </div>
    
    <div class="plugin-by-row">
        By {activePlugin.author}
    </div>
    
    <div class="plugin-desc-row">
        <div class="desc-text">{activePlugin.description}</div>
        <div class="plugin-actions">
            <Checkbox 
                checked={getPluginSettings(activePlugin.id).enabled}
                label="Enable Plugin"
                onchange={() => { togglePlugin(activePlugin.id); }}
            />
            <Button onclick={() => { handleDelete(activePlugin.id); }} variant="danger-ghost">
                <Trash2 size={12} /> Delete this plugin
            </Button>
        </div>
    </div>
</div>

<div class="properties-section">
    <h3>Settings</h3>
    
    <div class="properties-list" class:disabled={!getPluginSettings(activePlugin.id).enabled}>
        {#each activePlugin.properties as prop (prop.id)}
            <div class="property-row">
                <div class="prop-label-wrapper">
                    <span class="prop-name">{prop.name}</span>
                    {#if prop.tooltip}
                        <span class="info-icon" title={prop.tooltip}><Info size={12} /></span>
                    {/if}
                </div>
                
                <div class="prop-input">
                    {#if prop.kind.type === 'Float' || prop.kind.type === 'Int'}
                        {@const config = prop.kind.config}
                        <div class="number-input-wrapper">
                            <Input max={config.max} 
                                min={config.min} 
                                onchange={(e: Event) => { setProperty(activePlugin.id, prop.id, parseFloat((e.target as HTMLInputElement).value)); }} 
                                step={config.step}
                                type="number"
                                unit={config.unit}
                                value={getProperty(activePlugin.id, prop.id, config.default)}
                            />
                        </div>
                    {:else if prop.kind.type === 'Bool'}
                        {@const config = prop.kind.config}
                        <Checkbox
                            class="toggle-checkbox"
                            checked={getProperty(activePlugin.id, prop.id, config.default) as boolean}
                            onchange={(e: Event) => { setProperty(activePlugin.id, prop.id, (e.target as HTMLInputElement).checked); }}
                        />
                    {:else if prop.kind.type === 'Choice'}
                        {@const config = prop.kind.config}
                        <div class="theme-select-wrapper">
                            <Select 
                                onchange={(val) => { setProperty(activePlugin.id, prop.id, val as PropertyValue); }}
                                options={config.options.map((opt: string, idx: number) => ({ value: idx, label: opt }))}
                                value={getProperty(activePlugin.id, prop.id, config.default_index)}
                            />
                        </div>
                    {:else if prop.kind.type === 'String'}
                        {@const config = prop.kind.config}
                        <Input onchange={(e: Event) => { setProperty(activePlugin.id, prop.id, (e.target as HTMLInputElement).value); }}
                            type="text"
                            value={getProperty(activePlugin.id, prop.id, config.default)}
                        />
                    {/if}
                </div>
            </div>
        {/each}
        
        <div class="reset-row">
            <Button onclick={() => { resetToDefaults(activePlugin.id); }} variant="default">
                <RefreshCw size={14} /> Reset to Defaults
            </Button>
        </div>
    </div>
</div>

<style>
    .plugin-header-area {
        border-bottom: var(--border-width) solid var(--border);
        padding-bottom: var(--space-6);
        margin-bottom: var(--space-8);
    }
    
    .plugin-title-row {
        display: flex;
        align-items: center;
        gap: var(--space-3);
        margin-bottom: var(--space-2);
    }
    
    .header-icon {
        color: var(--text-active);
        display: flex;
    }
    
    .plugin-title-row h2 {
        font-size: var(--font-2xl);
        font-weight: var(--font-weight-bold);
        margin: 0;
        color: var(--text-active);
    }
    
    .badge {
        background: var(--bg-panel-hover);
        color: var(--text-main);
        font-size: var(--font-sm);
        padding: var(--space-0-5) var(--space-2);
        border-radius: var(--radius-md);
    }
    
    .plugin-by-row {
        font-size: var(--font-md);
        color: var(--text-muted);
        margin-bottom: var(--space-4);
    }
    
    .plugin-desc-row {
        display: flex;
        justify-content: space-between;
        align-items: flex-end;
    }
    
    .desc-text {
        font-size: var(--font-md);
        color: var(--text-muted);
        max-width: 60%;
        line-height: 1.5;
    }
    
    .plugin-actions {
        display: flex;
        flex-direction: column;
        align-items: flex-end;
        gap: var(--space-3);
    }

    .properties-section h3 {
        font-size: var(--font-xl);
        font-weight: var(--font-weight-bold);
        color: var(--text-active);
        margin-top: 0;
        margin-bottom: var(--space-6);
    }
    
    .properties-list {
        display: flex;
        flex-direction: column;
        gap: var(--space-4);
        max-width: 700px;
    }
    
    .properties-list.disabled {
        opacity: 0.4;
        pointer-events: none;
    }
    
    .property-row {
        display: flex;
        align-items: center;
        justify-content: space-between;
    }
    
    .prop-label-wrapper {
        display: flex;
        align-items: center;
        gap: var(--space-2);
    }
    
    .prop-name {
        font-size: var(--font-md);
        font-weight: var(--font-weight-bold);
        color: var(--text-main);
    }
    
    .info-icon {
        color: var(--text-muted);
        display: flex;
        cursor: help;
    }
    
    .prop-input {
        display: flex;
        align-items: center;
    }
    
    .number-input-wrapper {
        display: flex;
        align-items: center;
        width: 140px;
    }

    
    .theme-select-wrapper {
        width: 160px;
    }
    
    .reset-row {
        margin-top: var(--space-8);
        display: flex;
        justify-content: flex-start;
    }
    
</style>
