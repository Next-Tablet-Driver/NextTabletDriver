<script lang="ts">
    import { Settings as SettingsIcon } from "lucide-svelte";
    import type { MappingState } from "../../features/mapping/mapping-state.svelte";
    import Checkbox from "../../components/ui/Checkbox.svelte";

    const { mapping }: { mapping: MappingState } = $props();

    function updateConfig(): void {
        mapping.markDirty();
    }
</script>

<div class="section">
    <div class="section-header">
        <SettingsIcon size={20} />
        <h2>General Settings</h2>
    </div>
    
    {#if mapping.config}
    <div class="flex-column settings-group">
        <Checkbox 
            label="Run at startup" 
            onchange={updateConfig} 
            tooltip="Automatically start the driver in the background on system boot." 
            bind:checked={mapping.config.run_at_startup}
        />
        <Checkbox 
            label="System Tray when Minimize" 
            onchange={updateConfig} 
            tooltip="Hide to tray instead of taskbar. Unloads the UI to maximize performance." 
            bind:checked={mapping.config.system_tray_on_minimize}
        />
        <Checkbox 
            label="Force High Resolution Timer (0.5ms)" 
            onchange={updateConfig} 
            tooltip="Forces the Windows system timer to 0.5ms to reduce latency and jitter (Recommended)." 
            bind:checked={mapping.config.force_high_resolution_timer}
        />
        <Checkbox 
            checked={true} 
            label="Enable Anonymous Usage Statistics" 
            tooltip="Share anonymous hardware and crash data to help improve the driver."
        />
    </div>
    {/if}
</div>

<style>
    .section-header {
        display: flex;
        align-items: center;
        gap: var(--space-2);
        margin-bottom: var(--space-6);
        color: var(--text-active);
    }
    
    .section-header h2 {
        font-size: var(--font-xl);
        font-weight: var(--font-weight-bold);
        margin: 0;
    }

    .settings-group {
        padding-left: var(--space-7);
        gap: var(--space-4);
    }
    
    .flex-column {
        display: flex;
        flex-direction: column;
    }
</style>
