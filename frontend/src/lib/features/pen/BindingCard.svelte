<script lang="ts">
    import { Pencil } from "lucide-svelte";
    import type { MappingState } from "../../features/mapping/mapping-state.svelte";
    import Button from "../../components/ui/Button.svelte";

    const { mapping, type }: { mapping: MappingState, type: 'tip' | 'eraser' } = $props();

    function updateConfig(): void {
        mapping.markDirty();
    }
</script>

<div class="card">
    <div class="card-title">{type === 'tip' ? 'Tip Binding' : 'Eraser Binding'}</div>
    <div class="binding-input">
        {#if mapping.config !== null}
            {#if type === 'tip'}
                <input 
                    oninput={updateConfig} 
                    type="text" 
                    bind:value={mapping.config.tip_binding} 
                />
            {:else}
                <input 
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

    .binding-input {
        display: flex;
        align-items: center;
        background: var(--bg-input);
        border: var(--border-width) solid var(--border);
        border-radius: var(--radius-md);
        overflow: hidden;
    }

    .binding-input input {
        flex: 1;
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
