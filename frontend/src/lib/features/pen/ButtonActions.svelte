<script lang="ts">
    import { Pencil } from "lucide-svelte";
    import type { MappingState } from "../../features/mapping/mapping-state.svelte";
    import Button from "../../components/ui/Button.svelte";

    const { mapping }: { mapping: MappingState } = $props();

    function updateConfig(): void {
        mapping.markDirty();
    }
</script>

<div class="card buttons-card">
    <div class="section-header small">
        <svg fill="none" height="18" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2" viewBox="0 0 24 24" width="18"><rect height="20" rx="4" width="12" x="6" y="2"></rect><line x1="12" x2="12.01" y1="18" y2="18"></line></svg>
        <h3>Button Actions</h3>
    </div>
    
    {#if mapping.config}
        {#each mapping.config.pen_button_bindings as _binding, index (index)}
            <div class="binding-row">
                <label for={`pen_button_${  String(index)}`}>Pen Button {index + 1}</label>
                <div class="binding-input">
                    <input id={`pen_button_${  String(index)}`} oninput={updateConfig} type="text" bind:value={mapping.config.pen_button_bindings[index]} />
                    <Button variant="segment"><Pencil size={14} /></Button>
                </div>
            </div>
        {/each}
    {/if}
</div>

<style>
    .card {
        display: flex;
        flex-direction: column;
        gap: var(--space-4);
    }
    
    .buttons-card {
        grid-column: span 1;
        margin-top: var(--space-6);
    }

    .section-header {
        display: flex;
        align-items: center;
        gap: var(--space-2);
        color: var(--text-active);
    }
    
    .section-header.small {
        margin-bottom: var(--space-4);
    }
    
    .section-header.small h3 {
        font-size: var(--font-lg-plus);
        font-weight: var(--font-weight-bold);
        margin: 0;
    }

    .binding-row {
        display: flex;
        flex-direction: column;
        gap: var(--space-1);
        margin-bottom: var(--space-4);
    }
    
    .binding-row label {
        font-size: var(--font-md);
        color: var(--text-active);
        font-weight: var(--font-weight-medium);
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
