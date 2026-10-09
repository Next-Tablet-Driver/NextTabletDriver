<script lang="ts">
  import type { Snippet } from "svelte";

  const { children }: { children: Snippet } = $props();
</script>

<h3 class="subtitle">
  {@render children()}
</h3>

<style>
  .subtitle {
    color: var(--text-main);
    font-size: var(--font-lg);
    font-weight: var(--font-weight-medium);
    margin: 0 0 var(--space-2) 0;
  }
</style>
