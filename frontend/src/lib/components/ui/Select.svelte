<script lang="ts" module>
  export interface SelectOption<V = string | number> {
    value: V;
    label: string;
    disabled?: boolean;
  }
</script>

<script generics="T extends string | number" lang="ts">

  let {
    value = $bindable<T>(),
    options = [],
    onchange
  }: {
    value?: T;
    options?: (T | SelectOption<T>)[];
    onchange?: (value: T) => void;
  } = $props();

  const normalizedOptions = $derived(
    options.map((option): SelectOption<T> =>
      typeof option !== 'object'
        ? { value: option, label: String(option) }
        : option
    )
  );

  const selectedOption = $derived(
    normalizedOptions.find((option) => option.value === value)
  );

  const selectedLabel = $derived(
    selectedOption?.label ?? String(value)
  );

  let isOpen = $state(false);
  let selectRef = $state<HTMLDivElement | null>(null);
  let dropdownRef = $state<HTMLDivElement | null>(null);
  let side = $state<'bottom' | 'top'>('bottom');

  function calculateSide(): 'bottom' | 'top' {
    if (selectRef === null) return 'bottom';
    const rect = selectRef.getBoundingClientRect();
    const estimatedHeight = normalizedOptions.length * 32 + 10;
    const spaceBelow = window.innerHeight - rect.bottom;
    
    if (spaceBelow < estimatedHeight && rect.top > spaceBelow) {
      return 'top';
    }
    return 'bottom';
  }

  function toggleOpen(event: MouseEvent): void {
    event.preventDefault();
    if (!isOpen) {
      side = calculateSide();
    }
    isOpen = !isOpen;
  }

  function selectOption(option: SelectOption<T>, event: MouseEvent): void {
    event.preventDefault();

    if (option.disabled === true) return;

    value = option.value;
    isOpen = false;
    if (onchange !== undefined) {
      onchange(value);
    }
  }

  function handleOutsideClick(event: MouseEvent): void {
    if (isOpen && selectRef !== null && !selectRef.contains(event.target as Node)) {
      isOpen = false;
    }
  }

  function handleKeydown(event: KeyboardEvent): void {
    switch (event.key) {
      case 'Escape':
        event.preventDefault();
        isOpen = false;
        break;

      case 'Enter':
      case ' ':
        event.preventDefault();
        isOpen = !isOpen;
        break;

      case 'ArrowDown':
        event.preventDefault();

        if (!isOpen) {
          side = calculateSide();
          isOpen = true;
        } else {
          moveSelection(1);
        }
        break;

      case 'ArrowUp':
        event.preventDefault();

        if (!isOpen) {
          side = calculateSide();
          isOpen = true;
        } else {
          moveSelection(-1);
        }
        break;
    }
  }

  function moveSelection(direction: number): void {
    const enabledOptions = normalizedOptions.filter(
      (option) => option.disabled !== true
    );

    if (enabledOptions.length === 0) return;

    const currentIndex = enabledOptions.findIndex(
      (option) => option.value === value
    );

    const nextIndex =
      currentIndex === -1
        ? 0
        : (currentIndex + direction + enabledOptions.length) %
          enabledOptions.length;

    value = enabledOptions[nextIndex].value;
  }
</script>

<svelte:window onclick={handleOutsideClick} />

<div bind:this={selectRef} class="select">
  <button
    class="select-trigger"
    class:open={isOpen}
    aria-expanded={isOpen}
    aria-haspopup="listbox"
    onclick={toggleOpen}
    onkeydown={handleKeydown}
    type="button"
  >
    <span class="select-value">
      {selectedLabel}
    </span>

    <svg
      class="select-chevron"
      class:open={isOpen}
      aria-hidden="true"
      fill="none"
      viewBox="0 0 16 16"
    >
      <path
        d="M4 6L8 10L12 6"
        stroke="currentColor"
        stroke-linecap="round"
        stroke-linejoin="round"
        stroke-width="1.5"
      />
    </svg>
  </button>

  {#if isOpen}
    <div
      bind:this={dropdownRef}
      class="dropdown"
      data-side={side}
      onclick={(event) => { event.stopPropagation(); }}
      onkeydown={(event) => { event.stopPropagation(); }}
      role="listbox"
      tabindex="-1"
    >
      {#each normalizedOptions as option (option.value)}
        <button
          class="option"
          class:disabled={option.disabled}
          class:selected={option.value === value}
          aria-selected={option.value === value}
          disabled={option.disabled}
          onclick={(event) => { selectOption(option, event); }}
          role="option"
          type="button"
        >
          <span class="option-label">
            {option.label}
          </span>

          {#if option.value === value}
            <svg
              class="check"
              aria-hidden="true"
              fill="none"
              viewBox="0 0 16 16"
            >
              <path
                d="M3 8L6.5 11.5L13 4.5"
                stroke="currentColor"
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="1.8"
              />
            </svg>
          {/if}
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .select {
    position: relative;
    display: inline-block;
    width: 160px;
    font-family: inherit;
  }

  .select-trigger {
    width: 100%;
    height: var(--input-height);

    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);

    padding: var(--input-padding);

    box-sizing: border-box;

    border: var(--border-width) solid var(--border);
    border-radius: var(--input-radius);

    background: var(--bg-input);
    color: var(--item-text);

    font: inherit;
    font-size: var(--input-font-size);
    line-height: 1;

    cursor: pointer;
    outline: none;

    transition:
      background-color var(--transition-fast) ease,
      border-color var(--transition-fast) ease;
  }

  .select-trigger:hover {
    background: var(--bg-panel);
    border-color: var(--accent);
  }

  .select-trigger.open {
    border-color: var(--accent);
  }

  .select-trigger:focus-visible {
    border-color: var(--accent);
  }

  .select-value {
    min-width: 0;

    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .select-chevron {
    width: 14px;
    height: 14px;

    flex: 0 0 auto;

    color: var(--text-muted);

    transition:
      transform var(--transition-fast) ease,
      color var(--transition-fast) ease;
  }

  .select-chevron.open {
    transform: rotate(180deg);
    color: var(--text-main);
  }

  .dropdown {
    position: absolute;
    z-index: var(--z-dropdown);

    left: 0;
    width: 100%;
    padding: var(--menu-padding);

    display: flex;
    flex-direction: column;
    gap: var(--menu-gap);

    box-sizing: border-box;

    background: var(--menu-bg);
    border: var(--border-width) solid var(--menu-border);
    border-radius: var(--menu-radius-outer);

    animation: dropdown-in var(--transition-fast) ease-out;
  }

  .dropdown[data-side='bottom'] {
    top: calc(100% + 3px);
    transform-origin: top;
  }

  .dropdown[data-side='top'] {
    bottom: calc(100% + 3px);
    transform-origin: bottom;
  }

  .option {
    width: 100%;
    min-height: var(--item-min-height);

    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);

    padding: var(--item-padding);

    box-sizing: border-box;

    border: 0;
    border-radius: 0;

    background: transparent;
    color: var(--item-text);

    font: inherit;
    font-size: var(--item-font-size);
    line-height: 1.2;
    text-align: left;

    cursor: pointer;

    transition:
      background-color var(--transition-fast) ease,
      color var(--transition-fast) ease;
  }

  .option:first-of-type {
    border-top-left-radius: var(--menu-radius-inner);
    border-top-right-radius: var(--menu-radius-inner);
  }

  .option:last-of-type {
    border-bottom-left-radius: var(--menu-radius-inner);
    border-bottom-right-radius: var(--menu-radius-inner);
  }

  .option:hover:not(:disabled) {
    background: var(--bg-panel-hover);
  }

  .option.selected {
    background: var(--accent);
    color: var(--text-on-accent);
  }

  .option.selected:hover:not(:disabled) {
    background: var(--accent-hover);
  }

  .option:disabled {
    opacity: var(--disabled-opacity);
    cursor: default;
  }

  .option-label {
    min-width: 0;

    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .check {
    width: var(--item-icon-size);
    height: var(--item-icon-size);

    flex: 0 0 auto;

    color: var(--item-text-muted);
  }

  @keyframes dropdown-in {
    from {
      opacity: 0;
      transform: scaleY(0.96);
    }
    to {
      opacity: 1;
      transform: scaleY(1);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .select-trigger,
    .select-chevron,
    .option {
      transition: none;
    }

    .dropdown {
      animation: none;
    }
  }
</style>
