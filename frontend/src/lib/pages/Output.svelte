<script lang="ts">
  import type { MappingState } from '../features/mapping/mapping-state.svelte';
  
  import DisplayVisualizer from '../features/mapping/DisplayVisualizer.svelte';
  import TabletVisualizer from '../features/mapping/TabletVisualizer.svelte';

  const { mapping }: { mapping: MappingState } = $props();

  $effect(() => { mapping.subscribeToChanges(); });

    function handleGlobalKeydown(e: KeyboardEvent): void {
        if (!e.ctrlKey) return;

        const key = e.key.toLowerCase();
        
        if (key === "z") {
            e.preventDefault();
            if (e.shiftKey) {
                mapping.revertToInitial();
            } else {
                mapping.undo();
            }
        } else if (key === "y" && !e.shiftKey) {
            e.preventDefault();
            mapping.redo();
        } else if (key === "s" && !e.shiftKey) {
            e.preventDefault();
            void mapping.save();
        }
    }
</script>

<svelte:window onkeydown={handleGlobalKeydown} />

<div class="page">
  <DisplayVisualizer {mapping} />
  <TabletVisualizer {mapping} />
</div>

<style>
  .page {
      padding: var(--space-4);
      height: 100%;
      overflow-y: auto;
      display: flex;
      flex-direction: column;
      gap: var(--space-8);
  }
</style>
