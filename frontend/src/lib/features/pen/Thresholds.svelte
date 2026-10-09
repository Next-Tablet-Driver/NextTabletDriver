<script lang="ts">
    import type { MappingState } from "../../features/mapping/mapping-state.svelte";
    import Input from "../../components/ui/Input.svelte";

    const { mapping }: { mapping: MappingState } = $props();

    function updateConfig(): void {
        mapping.markDirty();
    }
</script>

<div class="card">
    <div class="card-title">Thresholds</div>
    <div class="flex-row">
        {#if mapping.config !== null}
            <div class="input-group">
                <Input label="Tip" max={1024} min={0} oninput={updateConfig} type="number" width="80px" bind:value={mapping.config.tip_threshold} />
            </div>
            <div class="input-group">
                <Input label="Eraser" max={1024} min={0} oninput={updateConfig} type="number" width="80px" bind:value={mapping.config.eraser_threshold} />
            </div>
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

    .flex-row {
        display: flex;
        gap: var(--space-4);
        align-items: center;
    }

    .input-group {
        display: flex;
        align-items: center;
        gap: var(--space-2);
    }
</style>
