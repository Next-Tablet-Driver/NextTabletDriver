<script lang="ts">
  import type { Snippet } from 'svelte';
  
  const { 
      variant = 'default',
      class: className = '',
      onclick,
      disabled = false,
      children,
      ...rest
  }: {
      variant?: 'default' | 'primary' | 'secondary' | 'tertiary' | 'ghost' | 'danger' | 'danger-ghost' | 'segment',
      class?: string,
      onclick?: (e: MouseEvent) => void,
      disabled?: boolean,
      children?: Snippet,
      [key: string]: unknown
  } = $props();
</script>

<button class="btn {variant} {className}" {disabled} {onclick} {...rest}>
  {#if children}
      {@render children()}
  {/if}
</button>

<style>
  .btn {
      display: inline-flex;
      align-items: center;
      justify-content: center;
      gap: var(--space-1-5);
      background-color: var(--bg-panel);
      color: var(--text-main);
      border: var(--border-width) solid var(--border);
      border-radius: var(--radius-md);
      padding: var(--space-1) var(--space-4);
      cursor: pointer;
      transition: all var(--transition-normal);
      font-size: var(--font-md);
      font-family: var(--font-sans);
      font-weight: var(--font-weight-medium);
  }

  .btn:hover:not(:disabled) {
      background-color: var(--bg-panel-hover);
  }

  .btn:active:not(:disabled) {
      transform: scale(0.98);
  }

  /* Disabled State */
  .btn:disabled {
      opacity: var(--disabled-opacity);
      cursor: not-allowed;
  }

  /* Primary */
  .btn.primary {
      background-color: var(--accent);
      border-color: var(--accent);
      color: var(--text-on-accent);
  }

  .btn.primary:hover:not(:disabled) {
      background-color: var(--accent-hover);
  }

  /* Secondary */
  .btn.secondary {
      background-color: transparent;
      border-color: var(--border);
      color: var(--text-main);
  }

  .btn.secondary:hover:not(:disabled) {
      background-color: var(--bg-panel);
      border-color: var(--text-muted);
  }

  /* Tertiary */
  .btn.tertiary {
      background-color: var(--bg-input);
      border-color: transparent;
      color: var(--text-main);
  }

  .btn.tertiary:hover:not(:disabled) {
      background-color: var(--bg-panel-hover);
  }

  /* Danger */
  .btn.danger {
      background-color: color-mix(in srgb, var(--danger) 10%, transparent);
      border-color: color-mix(in srgb, var(--danger) 30%, transparent);
      color: var(--danger);
  }

  .btn.danger:hover:not(:disabled) {
      background-color: color-mix(in srgb, var(--danger) 20%, transparent);
      border-color: color-mix(in srgb, var(--danger) 50%, transparent);
      color: var(--danger-hover);
  }

  /* Danger, outline only (destructive action that should not shout) */
  .btn.danger-ghost {
      background-color: transparent;
      border-color: color-mix(in srgb, var(--danger) 30%, transparent);
      color: var(--danger);
  }

  .btn.danger-ghost:hover:not(:disabled) {
      background-color: color-mix(in srgb, var(--danger) 10%, transparent);
  }

  /* Segment: an icon button attached to the end of an input group */
  .btn.segment {
      border: none;
      border-left: var(--border-width) solid var(--border);
      border-radius: 0;
      background-color: transparent;
      color: var(--text-muted);
      padding: 0 var(--space-3);
      height: 100%;
  }

  .btn.segment:hover:not(:disabled) {
      background-color: var(--bg-panel-hover);
      color: var(--text-active);
  }

  /* Ghost */
  .btn.ghost {
      background-color: transparent;
      border-color: transparent;
      color: var(--text-muted);
  }
  
  .btn.ghost:hover:not(:disabled) {
      background-color: var(--bg-panel);
      color: var(--text-active);
  }
</style>
