<script lang="ts">
  interface Props {
    value?: string | number;
    label?: string;
    unit?: string;
    type?: "text" | "number" | "password";
    step?: string | number;
    min?: string | number;
    max?: string | number;
    width?: string;
    precision?: number;
    oninput?: (e: Event) => void;
    onchange?: (e: Event) => void;
  }

  let {
    value = $bindable(),
    label = "",
    unit = "",
    type = "number",
    step = "1",
    min = undefined,
    max = undefined,
    width = "50px",
    precision = undefined,
    oninput = undefined,
    onchange = undefined
  }: Props = $props();

  let inputRef = $state<HTMLInputElement>();
  let labelRef = $state<HTMLElement>();

  const DRAG_THRESHOLD = 6; // px avant de considérer un swipe sur l'input

  function getStep(): number {
    const s = parseFloat(String(step));
    return Number.isNaN(s) ? 1 : s;
  }

  function currentNumber(): number {
    const n = parseFloat(inputRef?.value ?? String(value));
    return Number.isNaN(n) ? 0 : n;
  }

  function updateValue(newVal: number): void {
    if (precision !== undefined) {
      const factor = Math.pow(10, precision);
      newVal = Math.round(newVal * factor) / factor;
    } else {
      newVal = Math.round(newVal * 100000) / 100000;
    }

    if (min !== undefined && min !== "" && newVal < Number(min)) newVal = Number(min);
    if (max !== undefined && max !== "" && newVal > Number(max)) newVal = Number(max);

    value = newVal;

    if (inputRef !== undefined) {
      // On synchronise le DOM AVANT de dispatcher, sinon bind:value
      // et oninput lisent l'ancienne valeur.
      inputRef.value = precision !== undefined ? newVal.toFixed(precision) : String(newVal);
      inputRef.dispatchEvent(new Event("input", { bubbles: true }));
    }
  }

  function handleBlur(): void {
    if (type === 'number' && precision !== undefined && inputRef !== undefined && value !== undefined) {
      inputRef.value = Number(value).toFixed(precision);
    }
  }

  // Format initial value if needed
  $effect(() => {
    if (type === 'number' && precision !== undefined && inputRef !== undefined && document.activeElement !== inputRef) {
      inputRef.value = Number(value).toFixed(precision);
    }
  });

  // ── Molette / swipe trackpad (vertical ET horizontal) ─────────────
  function handleWheel(e: WheelEvent): void {
    if (type !== "number" || inputRef === undefined || document.activeElement !== inputRef) return;

    // Axe dominant : un swipe horizontal génère deltaX, pas deltaY
    const delta = Math.abs(e.deltaX) > Math.abs(e.deltaY) ? e.deltaX : e.deltaY;
    if (delta === 0) return;

    e.preventDefault();
    const direction = delta < 0 ? 1 : -1;
    updateValue(currentNumber() + direction * getStep());
  }

  $effect(() => {
    const el = inputRef;
    if (el === undefined) return;
    // non-passif, sinon preventDefault est ignoré
    el.addEventListener("wheel", handleWheel, { passive: false });
    return () => { el.removeEventListener("wheel", handleWheel); };
  });

  // ── Swipe / drag (pointer events natifs) ──────────────────────────
  // Sur le label : drag immédiat (souris, tactile, stylet).
  // Sur l'input : swipe horizontal tactile uniquement (après un seuil),
  // pour ne pas gêner la sélection de texte à la souris.
  let pending = false;
  let dragging = false;
  let startX = 0;
  let startY = 0;
  let startValue = 0;

  function attachDrag(el: HTMLElement, fromLabel: boolean): () => void {
    const beginDrag = (e: PointerEvent): void => {
      dragging = true;
      el.setPointerCapture(e.pointerId);
      if (!fromLabel && inputRef !== undefined) inputRef.blur(); // évite le clavier / curseur pendant le swipe
    };

    const onDown = (e: PointerEvent): void => {
      if (type !== "number") return;
      if (e.pointerType === "mouse" && (!fromLabel || e.button !== 0)) return;

      pending = true;
      dragging = false;
      startX = e.clientX;
      startY = e.clientY;
      startValue = currentNumber();

      if (fromLabel) {
        beginDrag(e);
        e.preventDefault(); // pas de sélection de texte
      }
    };

    const onMove = (e: PointerEvent): void => {
      if (!pending) return;
      const dx = e.clientX - startX;

      if (!dragging) {
        const dy = e.clientY - startY;
        if (Math.abs(dx) < DRAG_THRESHOLD || Math.abs(dx) < Math.abs(dy)) return;
        beginDrag(e);
      }

      const multiplier = e.shiftKey ? 10 : 1;
      updateValue(startValue + dx * getStep() * multiplier);
    };

    const onUp = (e: PointerEvent): void => {
      if (!pending) return;
      pending = false;
      if (dragging) {
        dragging = false;
        try {
          el.releasePointerCapture(e.pointerId);
        } catch {
          /* déjà relâché */
        }
      }
    };

    el.addEventListener("pointerdown", onDown);
    el.addEventListener("pointermove", onMove);
    el.addEventListener("pointerup", onUp);
    el.addEventListener("pointercancel", onUp);

    return () => {
      el.removeEventListener("pointerdown", onDown);
      el.removeEventListener("pointermove", onMove);
      el.removeEventListener("pointerup", onUp);
      el.removeEventListener("pointercancel", onUp);
    };
  }

  $effect(() => {
    const el = labelRef;
    if (el === undefined) return;
    return attachDrag(el, true);
  });

  $effect(() => {
    const el = inputRef;
    if (el === undefined) return;
    return attachDrag(el, false);
  });
</script>

<div class="measure-input-group">
  {#if label}
    <span
      bind:this={labelRef}
      class="label"
      class:draggable={type === 'number'}
    >
      {label}
    </span>
  {/if}
  <div class="input-wrapper">
    <input
      bind:this={inputRef}
      style:width
      class:is-number={type === 'number'}
      {max}
      {min}
      onblur={handleBlur}
      {onchange}
      {oninput}
      {step}
      {type}
      bind:value
    />
    {#if unit}
      <span class="unit">{unit}</span>
    {/if}
  </div>
</div>

<style>
  .measure-input-group {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    font-family: var(--font-sans);
  }

  /* Label */
  .label {
    font-size: var(--font-md);
    font-weight: var(--font-weight-medium);
    color: var(--text-main);
    user-select: none;
    -webkit-user-select: none;
    white-space: nowrap;
    transition: color var(--transition-normal) ease;
  }

  .label.draggable {
    cursor: ew-resize;
    touch-action: none;
  }

  .label.draggable:hover {
    color: var(--text-active);
  }

  .label.draggable:focus-visible {
    outline: 2px solid var(--input-border-focus);
    outline-offset: 2px;
    border-radius: var(--radius-sm-plus);
  }

  /* Champ */
  .input-wrapper {
    display: flex;
    align-items: baseline;
    gap: var(--space-1);
    cursor: text;
    background-color: var(--input-bg);
    border: var(--border-width) solid transparent;
    border-radius: var(--input-radius);
    padding: var(--input-padding);
    transition:
      border-color var(--transition-normal) ease,
      background-color var(--transition-normal) ease,
      box-shadow var(--transition-normal) ease;
  }

  .input-wrapper:hover {
    background-color: var(--input-bg-hover);
    border-color: var(--input-border);
  }

  .input-wrapper:focus-within {
    background-color: var(--input-bg-focus);
    border-color: var(--input-border-focus);
    box-shadow: 0 0 0 1px var(--input-border-focus);
  }

  /* Input */
  .input-wrapper input {
    min-width: 0;
    padding: 0;
    background: transparent;
    color: var(--text-main);
    font-family: inherit;
    font-size: var(--input-font-size);
    font-weight: var(--font-weight-medium);
    border: none;
    outline: none;
    caret-color: var(--input-border-focus);
  }

  .input-wrapper input::placeholder {
    color: var(--text-muted);
    opacity: 0.5;
  }

  .input-wrapper input::selection {
    background-color: color-mix(in srgb, var(--input-border-focus) 35%, transparent);
  }

  .input-wrapper input.is-number {
    color: var(--text-active);
    text-align: right;
    touch-action: pan-y;
    font-variant-numeric: tabular-nums;
    -moz-appearance: textfield;
    appearance: textfield;
  }

  .input-wrapper input[type=number]::-webkit-inner-spin-button,
  .input-wrapper input[type=number]::-webkit-outer-spin-button {
    -webkit-appearance: none;
    margin: 0;
  }

  .unit {
    font-size: var(--font-sm);
    font-family: var(--font-sans);
    color: var(--text-muted);
    opacity: 0.7;
    user-select: none;
    -webkit-user-select: none;
    white-space: nowrap;
    transition: opacity var(--transition-normal) ease;
  }

  .input-wrapper:focus-within .unit {
    opacity: 1;
  }

  @media (prefers-reduced-motion: reduce) {
    .label,
    .input-wrapper,
    .unit {
      transition: none;
    }
  }
</style>