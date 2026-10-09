<script lang="ts">
    import { Pencil } from "lucide-svelte";
    import type { MappingState } from "../features/mapping/mapping-state.svelte";
    
    import Thresholds from "../features/pen/Thresholds.svelte";
    import HardwareToggles from "../features/pen/HardwareToggles.svelte";
    import BindingCard from "../features/pen/BindingCard.svelte";
    import PressureCurve from "../features/pen/PressureCurve.svelte";
    import ButtonActions from "../features/pen/ButtonActions.svelte";

    const { mapping }: { mapping: MappingState } = $props();
</script>

<div class="page-container">
    <div class="section-header">
        <Pencil size={20} />
        <h2>Pen Settings</h2>
    </div>

    {#if mapping.config}
        <div class="grid-layout">
            <Thresholds {mapping} />
            <HardwareToggles {mapping} />
            <BindingCard {mapping} type="tip" />
            <BindingCard {mapping} type="eraser" />
            <PressureCurve {mapping} />
            <ButtonActions {mapping} />
        </div>
    {/if}
</div>

<style>
    .page-container {
        padding: var(--space-8);
        height: 100%;
        overflow-y: auto;
    }

    .section-header {
        display: flex;
        align-items: center;
        gap: var(--space-2);
        margin-bottom: var(--space-8);
        color: var(--text-active);
    }
    
    .section-header h2 {
        font-size: var(--font-xl-plus);
        font-weight: var(--font-weight-bold);
        margin: 0;
    }
    
    .grid-layout {
        display: grid;
        grid-template-columns: repeat(2, 1fr);
        gap: var(--space-8);
        max-width: 900px;
    }
</style>
