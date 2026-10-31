<script lang="ts">
    import type { MappingState } from "../features/mapping/mapping-state.svelte";
    import Title from "../components/ui/Title.svelte";

    import WindowsInkCard from "../features/drawing/WindowsInkCard.svelte";
    import PenTestPad from "../features/drawing/PenTestPad.svelte";

    const { mapping }: { mapping: MappingState } = $props();
</script>

<div class="page">
    {#if mapping.config}
        <div class="section">
            <Title>Pen Mode</Title>
            <WindowsInkCard {mapping} />
        </div>

        <div class="section">
            <Title>Test Pad</Title>
            <PenTestPad />
            <p class="note">The pressure curve, the thresholds and the other pen settings are on the Pen Settings tab.</p>
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

    .note {
        margin: 0;
        color: var(--text-muted);
        font-size: var(--font-base);
    }
</style>
