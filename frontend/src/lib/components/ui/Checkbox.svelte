<script lang="ts">
  import Tooltip from "./Tooltip.svelte";

  let {
      checked = $bindable(false), 
      label = "",
      tooltip = "",
      ...rest
  } = $props();
</script>

{#if tooltip}
<Tooltip text={tooltip}>
  <label class="checkbox-label">
    <input type="checkbox" bind:checked {...rest} />
    {label}
  </label>
</Tooltip>
{:else}
<label class="checkbox-label">
  <input type="checkbox" bind:checked {...rest} />
  {label}
</label>
{/if}

<style>
  .checkbox-label {
      color: var(--text-main);
      font-size: var(--font-md);
      line-height: 1;
      display: flex;
      align-items: center;
      gap: var(--space-2);
      cursor: pointer;
      user-select: none;
  }

  input[type="checkbox"] {
      appearance: none;
      -webkit-appearance: none;
      display: inline-flex;
      position: relative;
      flex-shrink: 0;
      width: 16px;
      height: 16px;
      margin: 0;
      background-color: var(--bg-input);
      border: var(--border-width) solid var(--border);
      border-radius: var(--radius-sm);
      cursor: pointer;
      transition: background-color var(--transition-normal) ease,
                  border-color var(--transition-normal) ease,
                  transform var(--transition-fast) ease;
      top: 1px;
  }

  /* Hover */
  input[type="checkbox"]:hover:not(:disabled) {
      border-color: var(--accent);
  }

  /* Pressed Effect */
  input[type="checkbox"]:active:not(:disabled) {
      transform: scale(0.95);
  }

  /* Checked */
  input[type="checkbox"]:checked {
      background-color: var(--accent);
      border-color: var(--accent);
  }

  /* Check mark */
  input[type="checkbox"]::after {
      content: "";
      position: absolute;
      top: 50%;
      left: 50%;
      width: 4px;
      height: 8px;
      border: solid var(--text-active);
      border-width: 0 2px 2px 0;
      transform: translate(-50%, -60%) rotate(45deg) scale(0);
      opacity: 0;
      transition: transform var(--transition-normal) cubic-bezier(0.3, 0.7, 0.4, 1.3),
                  opacity var(--transition-fast) ease;
  }

  input[type="checkbox"]:checked::after {
      transform: translate(-50%, -60%) rotate(45deg) scale(1);
      opacity: 1;
  }

  /* Keyboard focus */
  input[type="checkbox"]:focus-visible {
      outline: var(--outline-width) solid var(--accent);
      outline-offset: 2px;
  }

  input[type="checkbox"]:disabled {
      opacity: var(--disabled-opacity);
      cursor: not-allowed;
  }
</style>