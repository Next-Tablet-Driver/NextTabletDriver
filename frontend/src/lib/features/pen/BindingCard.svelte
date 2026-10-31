<script lang="ts">
    import { Pencil } from "lucide-svelte";
    import type { MappingState } from "../../features/mapping/mapping-state.svelte";
    import Button from "../../components/ui/Button.svelte";

    const { mapping, type }: { mapping: MappingState, type: 'tip' | 'eraser' } = $props();

    function updateConfig(): void {
        mapping.markDirty();
    }
</script>

<div class="binding">
    <p class="title">{type === 'tip' ? 'Tip Binding' : 'Eraser Binding'}</p>
    <div class="binding-input">
        {#if mapping.config !== null}
            {#if type === 'tip'}
                <input
                    aria-label="Tip binding"
                    oninput={updateConfig}
                    type="text"
                    bind:value={mapping.config.tip_binding}
                />
            {:else}
                <input
                    aria-label="Eraser binding"
                    oninput={updateConfig}
                    type="text"
                    bind:value={mapping.config.eraser_binding}
                />
            {/if}
        {/if}
        <Button aria-label={`Edit ${type} Binding`} variant="segment"><Pencil size={14} /></Button>
    </div>
</div>

<style>
    .binding {
        display: flex;
        flex-direction: column;
        gap: var(--space-1-5);
    }

    .title {
        margin: 0;
        color: var(--text-main);
        font-size: var(--font-base);
    }

    .binding-input {
        display: flex;
        align-items: center;
        background: var(--bg-input);
        border: var(--border-width) solid var(--border);
        border-radius: var(--radius-md);
        overflow: hidden;
        transition: border-color var(--transition-normal) var(--ease-emphasized);
    }

    .binding-input:focus-within {
        border-color: var(--input-border-focus);
    }

    .binding-input input {
        flex: 1;
        min-width: 0;
        background: transparent;
        border: none;
        color: var(--text-muted);
        padding: var(--space-2) var(--space-3);
        font-size: var(--font-md);
        text-align: left;
        outline: none;
    }

    .binding-input input:focus {
        color: var(--text-active);
    }
</style>
