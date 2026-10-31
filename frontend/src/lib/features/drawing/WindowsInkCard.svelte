<script lang="ts">
    import type { MappingState } from "../mapping/mapping-state.svelte";

    const { mapping }: { mapping: MappingState } = $props();

    const ink = $derived(mapping.config?.windows_ink ?? false);
    const relative = $derived(mapping.config !== null && mapping.config.mode !== "Absolute");

    function choose(windowsInk: boolean): void {
        if (mapping.config === null || mapping.config.windows_ink === windowsInk) return;
        mapping.config.windows_ink = windowsInk;
        mapping.markDirty();
    }
</script>

{#if mapping.config !== null}
    <div class="mode">
        <div class="segmented" aria-label="Pen mode" role="radiogroup">
            <button class="segment" class:selected={!ink} aria-checked={!ink} onclick={() => { choose(false); }} role="radio" type="button">Mouse</button>
            <button class="segment" class:selected={ink} aria-checked={ink} onclick={() => { choose(true); }} role="radio" type="button">Windows Ink</button>
        </div>

        {#if ink && relative}
            <p class="note warning" role="status">Relative mode: the pen still acts as a mouse.</p>
        {:else if ink}
            <p class="note">Pressure, tilt, hover and the eraser reach your programs. Needs Windows 10 (1809) or later, and Absolute mode.</p>
        {:else}
            <p class="note">Cursor and clicks only, with the lowest latency. Keep it for osu! and games.</p>
        {/if}
    </div>
{/if}

<style>
    .mode {
        display: flex;
        flex-wrap: wrap;
        gap: var(--space-2) var(--space-4);
        align-items: center;
    }

    .segmented {
        display: inline-flex;
        flex-shrink: 0;
        overflow: hidden;
        border: var(--border-width) solid var(--border);
        border-radius: var(--radius-md);
    }

    .segment {
        padding: var(--space-1-5) var(--space-4);
        color: var(--text-main);
        font: inherit;
        font-size: var(--font-md);
        background: transparent;
        border: none;
        cursor: pointer;
        transition-duration: var(--transition-normal);
        transition-property: background-color, color, scale;
        transition-timing-function: var(--ease-emphasized);
    }

    .segment + .segment {
        border-left: var(--border-width) solid var(--border);
    }

    .segment:hover:not(.selected) {
        color: var(--text-active);
        background: var(--overlay-5);
    }

    .segment:active {
        scale: 0.96;
    }

    .segment.selected {
        color: var(--text-on-accent);
        background: var(--accent);
    }

    .note {
        margin: 0;
        color: var(--text-muted);
        font-size: var(--font-base);
        text-wrap: pretty;
    }

    .warning {
        color: var(--warning);
    }
</style>
