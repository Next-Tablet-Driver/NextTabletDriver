<script lang="ts">
  import type { Snippet } from "svelte";

  const { children }: { children: Snippet } = $props();
</script>

<h2 class="title">
  {@render children()}
</h2>

<style>
  .title {
    color: var(--text-active);
    font-size: var(--font-xl);
    font-weight: var(--font-weight-bold);
  }
</style>
