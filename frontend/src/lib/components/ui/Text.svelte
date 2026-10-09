<script lang="ts">
  import type { Snippet } from 'svelte';

  const { 
      muted = false,
      children
  }: {
      muted?: boolean,
      children: Snippet
  } = $props();
</script>

<p class="text" class:muted>
  {@render children()}
</p>

<style>
  .text {
    color: var(--text-main);
    font-size: var(--font-md);
    margin: 0;
    line-height: 1.4;
  }

  .text.muted {
    color: var(--text-muted);
  }
</style>
