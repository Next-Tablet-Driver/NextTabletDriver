<script lang="ts">
    import type { MappingState } from "../features/mapping/mapping-state.svelte";
    import Title from "../components/ui/Title.svelte";

    import Thresholds from "../features/pen/Thresholds.svelte";
    import HardwareToggles from "../features/pen/HardwareToggles.svelte";
    import BindingCard from "../features/pen/BindingCard.svelte";
    import PressureCurve from "../features/pen/PressureCurve.svelte";
    import ButtonActions from "../features/pen/ButtonActions.svelte";

    const { mapping }: { mapping: MappingState } = $props();
</script>

<div class="page">
    {#if mapping.config}
        <div class="section">
            <Title>Pen</Title>
            <div class="controls">
                <HardwareToggles {mapping} />
                <Thresholds {mapping} />
            </div>
        </div>

        <div class="section">
            <Title>Pressure Curve</Title>
            <PressureCurve {mapping} />
        </div>

        <div class="section">
            <Title>Buttons</Title>
            <ButtonActions {mapping} />
        </div>

        <div class="section">
            <Title>Tip and Eraser</Title>
            <p class="note">Saved, but not applied yet.</p>
            <div class="pair">
                <BindingCard {mapping} type="tip" />
                <BindingCard {mapping} type="eraser" />
            </div>
        </div>
    {/if}
</div>

<style>
    .page {
        display: flex;
        flex-direction: column;
        gap: var(--space-8);
        height: 100%;
        padding: var(--space-4);
        overflow-y: auto;
    }

    .section {
        display: flex;
        flex-direction: column;
        gap: var(--space-2);
    }

    .controls {
        display: flex;
        flex-direction: column;
        gap: var(--space-3);
    }

    .note {
        margin: 0;
        color: var(--text-muted);
        font-size: var(--font-base);
    }

    .pair {
        display: grid;
        grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
        gap: var(--space-4);
    }
</style>
