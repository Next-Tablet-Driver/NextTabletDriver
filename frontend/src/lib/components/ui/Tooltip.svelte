<script lang="ts">
  import type { Snippet } from "svelte";

  interface Props {
    text?: string;
    delay?: number;
    children?: Snippet;
    disabled?: boolean;
  }

  const { 
    text = "", 
    delay = 1000, 
    children,
    disabled = false
  }: Props = $props();

  let visible = $state(false);
  let timer: number | undefined;
  
  let x = $state(0);
  let y = $state(0);

  function handleMouseEnter(event: MouseEvent): void {
    if (text === "" || disabled) return;
    x = event.clientX;
    y = event.clientY;
    
    timer = window.setTimeout(() => {
      visible = true;
    }, delay);
  }

  function handleMouseLeave(): void {
    if (timer !== undefined) clearTimeout(timer);
    visible = false;
  }
  
  function handleMouseMove(event: MouseEvent): void {
    if (!visible && timer !== undefined) {
      x = event.clientX;
      y = event.clientY;
    }
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div 
  class="tooltip-wrapper" 
  onmouseenter={handleMouseEnter} 
  onmouseleave={handleMouseLeave}
  onmousemove={handleMouseMove}
>
  {@render children?.()}
  
  {#if visible && !disabled && text !== ""}
    <div style:top="{y + 20}px" style:left="{x + 10}px" class="tooltip-content">
      {text}
    </div>
  {/if}
</div>

<style>
  .tooltip-wrapper {
    display: inline-flex;
    /* This ensures it doesn't break the layout of wrapped elements */
    width: fit-content;
  }

  .tooltip-content {
    position: fixed;
    z-index: var(--z-tooltip);
    background-color: var(--bg-panel);
    color: var(--text-main);
    padding: var(--space-2) var(--space-4);
    border: var(--border-width) solid var(--border);
    border-radius: var(--radius-sm);
    font-size: var(--font-md);
    line-height: 1.4;
    max-width: 300px;
    box-shadow: var(--shadow-popover);
    pointer-events: none;
    animation: fadeIn var(--transition-normal) ease-out forwards;
    white-space: pre-wrap; /* Allows formatting with newlines */
  }
  
  @keyframes fadeIn {
    from {
      opacity: 0;
      transform: translateY(-4px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }
</style>
