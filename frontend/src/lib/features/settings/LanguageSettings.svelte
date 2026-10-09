<script lang="ts">
    import { Languages } from "lucide-svelte";
    import type { MappingState } from "../../features/mapping/mapping-state.svelte";
    import Select from "../../components/ui/Select.svelte";

    const { mapping }: { mapping: MappingState } = $props();

    function updateConfig(): void {
        mapping.markDirty();
    }
</script>

<div class="section">
    <div class="section-header">
        <Languages size={20} />
        <h2>Language</h2>
    </div>
    
    {#if mapping.config}
    <div class="flex-column settings-group">
        <div class="input-group">
            <label for="interface_language">Interface Language</label>
            <div class="theme-select-wrapper">
                <Select onchange={updateConfig} options={["English", "Français"]} bind:value={mapping.config.language} />
            </div>
        </div>
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

    .input-group {
        display: flex;
        align-items: center;
        gap: var(--space-4);
    }
    
    .input-group label {
        font-size: var(--font-md);
        color: var(--text-active);
        font-weight: var(--font-weight-bold);
        min-width: 120px;
    }

    .theme-select-wrapper {
        width: 160px;
    }
</style>
