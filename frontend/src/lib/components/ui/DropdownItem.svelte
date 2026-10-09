<script lang="ts">
  import { ChevronRight } from 'lucide-svelte';
  import type { Snippet } from 'svelte';
  
  const { 
    children, 
    rightContent, // Snippet for right side (e.g. arrow, shortcut)
    disabled = false,
    hasSubmenu = false,
    onclick,
    onmouseenter,
    onmouseleave
  } = $props<{
    children?: Snippet;
    rightContent?: Snippet;
    disabled?: boolean;
    hasSubmenu?: boolean;
    onclick?: (e: MouseEvent) => void;
    onmouseenter?: (e: MouseEvent) => void;
    onmouseleave?: (e: MouseEvent) => void;
  }>();

  function handleClick(e: MouseEvent): void {
    if (disabled) {
        e.stopPropagation();
        return;
    }
    if (onclick !== undefined) onclick(e);
  }

  function handleKeyDown(e: KeyboardEvent): void {
      if (e.key === 'Enter' || e.key === ' ') {
          if (disabled) return;
          e.preventDefault();
          if (onclick !== undefined) onclick(e as unknown as MouseEvent);
      }
  }
</script>
<div 
  class="dropdown-item" 
  class:disabled 
  class:has-submenu={hasSubmenu}
  onclick={handleClick}
  onkeydown={handleKeyDown}
  {onmouseenter}
  {onmouseleave}
  role="menuitem"
  tabindex={disabled ? -1 : 0}
>
  <span class="content">{#if children}{@render children()}{/if}</span>
  {#if rightContent}
      <span class="right">{@render rightContent()}</span>
  {:else if hasSubmenu}
      <span class="arrow"><ChevronRight size={16} /></span>
  {/if}
</div>

<style>
  .dropdown-item {
    min-height: var(--item-min-height);
    padding: var(--item-padding);
    font-size: var(--item-font-size);
    color: var(--item-text);
    cursor: pointer;
    display: flex;
    justify-content: space-between;
    align-items: center;
    user-select: none;
    position: relative;
    border-radius: 0;
  }

  .dropdown-item:first-of-type {
    border-top-left-radius: var(--menu-radius-inner);
    border-top-right-radius: var(--menu-radius-inner);
  }

  .dropdown-item:last-of-type {
    border-bottom-left-radius: var(--menu-radius-inner);
    border-bottom-right-radius: var(--menu-radius-inner);
  }

  .dropdown-item:hover:not(:has(:global([role="menu"]:hover))) {
    background-color: var(--item-bg-selected);
    color: var(--item-text-active);
  }

  .dropdown-item.disabled {
    color: var(--item-text-muted);
    cursor: default;
  }
  
  .dropdown-item.disabled:hover {
    background-color: transparent;
    color: var(--item-text-muted);
  }

  .arrow {
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--item-text-muted);
    margin-left: var(--space-2);
  }

  .dropdown-item:hover:not(:has(:global([role="menu"]:hover))) .arrow {
    color: var(--item-text-active);
  }

  .dropdown-item:focus-visible {
    outline: var(--outline-width) solid var(--accent);
    outline-offset: -2px;
  }

  .content {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  :global(.dropdown-item svg) {
    flex-shrink: 0;
  }
</style>
