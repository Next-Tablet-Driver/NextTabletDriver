<script lang="ts">
    import type { MappingState } from "../../features/mapping/mapping-state.svelte";
    import Checkbox from "../../components/ui/Checkbox.svelte";

    const { mapping }: { mapping: MappingState } = $props();

    function updateConfig(): void {
        mapping.markDirty();
    }
</script>

<div class="card">
    <div class="card-title">Hardware Toggles</div>
    <div class="flex-column">
        {#if mapping.config !== null}
            <Checkbox label="Disable Pressure" onchange={updateConfig} bind:checked={mapping.config.disable_pressure} />
            <Checkbox label="Disable Tilt" onchange={updateConfig} bind:checked={mapping.config.disable_tilt} />
        {/if}
    </div>
</div>

<style>
    .card {
        display: flex;
        flex-direction: column;
        gap: var(--space-4);
    }
    
    .card-title {
        font-size: var(--font-md);
        font-weight: var(--font-weight-bold);
        color: var(--text-muted);
    }

    .flex-column {
        display: flex;
        flex-direction: column;
        gap: var(--space-2);
    }
</style>
