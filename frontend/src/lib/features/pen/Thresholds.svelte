<script lang="ts">
    import type { MappingState } from "../../features/mapping/mapping-state.svelte";
    import Input from "../../components/ui/Input.svelte";
    import Tooltip from "../../components/ui/Tooltip.svelte";

    const { mapping }: { mapping: MappingState } = $props();

    function updateConfig(): void {
        mapping.markDirty();
    }
</script>

{#if mapping.config !== null}
    <div class="input-row">
        <Tooltip text="How hard you press before the pen tip counts as touching, as a share of the maximum pressure.">
            <Input label="Tip threshold" max={100} min={0} oninput={updateConfig} type="number" unit="%" width="80px" bind:value={mapping.config.tip_threshold} />
        </Tooltip>
        <Tooltip text="The same for the eraser end of the pen.">
            <Input label="Eraser threshold" max={100} min={0} oninput={updateConfig} type="number" unit="%" width="80px" bind:value={mapping.config.eraser_threshold} />
        </Tooltip>
    </div>
{/if}

<style>
    .input-row {
        display: flex;
        flex-wrap: wrap;
        gap: var(--space-4);
    }
</style>
