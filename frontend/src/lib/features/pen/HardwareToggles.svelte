<script lang="ts">
    import type { MappingState } from "../../features/mapping/mapping-state.svelte";
    import Checkbox from "../../components/ui/Checkbox.svelte";

    const { mapping }: { mapping: MappingState } = $props();

    function updateConfig(): void {
        mapping.markDirty();
    }
</script>

{#if mapping.config !== null}
    <div class="checkbox-row">
        <Checkbox
            label="Disable Pressure"
            onchange={updateConfig}
            tooltip="The pen reports full pressure whenever it touches the tablet."
            bind:checked={mapping.config.disable_pressure}
        />
        <Checkbox
            label="Disable Tilt"
            onchange={updateConfig}
            tooltip="Ignore the angle of the pen."
            bind:checked={mapping.config.disable_tilt}
        />
    </div>
{/if}

<style>
    .checkbox-row {
        display: flex;
        flex-wrap: wrap;
        gap: var(--space-4);
    }
</style>
