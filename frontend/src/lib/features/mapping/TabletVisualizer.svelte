<script lang="ts">
  import { onMount } from "svelte";
  import { getTabletStatus } from "../../infrastructure/tauri/commands";
  import { events } from "../../infrastructure/events";
  import type { MappingState } from "./mapping-state.svelte";
  import Title from "../../components/ui/Title.svelte";
  import Checkbox from "../../components/ui/Checkbox.svelte";
  import Input from "../../components/ui/Input.svelte";

  const OSU_PLAYFIELD_W = 1316.0;
  const OSU_PLAYFIELD_H = 1080.0;
  const OSU_PLAYFIELD_ASPECT = OSU_PLAYFIELD_W / OSU_PLAYFIELD_H;
  const OSU_MENU_INNER_H = 1028.0;
  const OSU_MENU_Y_OFFSET = 496.0;
  const DEBOUNCE_PEN_HIDE_MS = 500;

  const { mapping }: { mapping: MappingState } = $props();

  let physW = $state(0.0);
  let physH = $state(0.0);
  let tabletMaxX = $state(0.0);
  let tabletMaxY = $state(0.0);

  let tabletAreaW = $state(0);
  let tabletAreaH = $state(0);
  const tabletScale = $derived(
    tabletAreaW > 0 && tabletAreaH > 0
      ? Math.min((tabletAreaW * 0.95) / physW, (tabletAreaH * 0.95) / physH)
      : 0,
  );
  const tabletCanvasW = $derived(physW * tabletScale);
  const tabletCanvasH = $derived(physH * tabletScale);

  let currentPenX = $state(0);
  let currentPenY = $state(0);
  let isPenActive = $state(false);
  let penTimeout: ReturnType<typeof setTimeout>;
  let cleanupUnlisten: (() => void) | null | undefined = null;
  let cleanupDeviceUnlisten: (() => void) | null | undefined = null;

  onMount(() => {
    void (async (): Promise<void> => {
      const updateTabletData = async (): Promise<void> => {
        try {
          const tabletStatus = await getTabletStatus();
          if (tabletStatus.width > 0 && tabletStatus.height > 0) {
            physW = tabletStatus.width;
            physH = tabletStatus.height;
            tabletMaxX = tabletStatus.max_x ?? tabletMaxX;
            tabletMaxY = tabletStatus.max_y ?? tabletMaxY;
          }
        } catch (e) {
          console.warn("Could not load tablet data", e);
        }
      };

      await updateTabletData();
      cleanupDeviceUnlisten = await events.onDeviceChanged(() => { void updateTabletData(); });

      cleanupUnlisten = await events.onTabletEvent((data): void => {
        const payload = data as { status?: string, x: number, y: number } | null;
        if (payload?.status !== undefined) {
          clearTimeout(penTimeout);

          const statusStr = payload.status.toLowerCase();
          const isActuallyActive =
            statusStr.includes("hover") ||
            statusStr.includes("contact") ||
            statusStr.includes("pen") ||
            statusStr.includes("active");

          if (!isActuallyActive) {
            isPenActive = false;
          } else {
            currentPenX = payload.x;
            currentPenY = payload.y;
            isPenActive = true;

            penTimeout = setTimeout(() => {
              isPenActive = false;
            }, DEBOUNCE_PEN_HIDE_MS);
          }
        }
      });
    })();

    return () => {
      if (cleanupUnlisten !== null && cleanupUnlisten !== undefined) cleanupUnlisten();
      if (cleanupDeviceUnlisten !== null && cleanupDeviceUnlisten !== undefined) cleanupDeviceUnlisten();
      clearTimeout(penTimeout);
    };
  });

  let vizTabletContainer: HTMLDivElement | undefined = $state();
  let isDraggingTablet = false;
  let startTabletMouseX = 0,
    startTabletMouseY = 0;
  let startTabletX = 0,
    startTabletY = 0;

  function onTabletPointerDown(e: PointerEvent): void {
    isDraggingTablet = true;
    startTabletMouseX = e.clientX;
    startTabletMouseY = e.clientY;
    startTabletX = mapping.tabletX;
    startTabletY = mapping.tabletY;
    (e.target as HTMLElement).setPointerCapture(e.pointerId);
  }

  function onTabletPointerMove(e: PointerEvent): void {
    if (!isDraggingTablet || vizTabletContainer === undefined) return;

    const canvas = vizTabletContainer.querySelector(
      ".tablet-canvas",
    );
    if (canvas === null) return;

    const canvasWidth = canvas.getBoundingClientRect().width;
    const scale = physW / canvasWidth;

    const dx = (e.clientX - startTabletMouseX) * scale;
    const dy = (e.clientY - startTabletMouseY) * scale;

    mapping.tabletX =
      Math.round(
        Math.max(
          mapping.tabletW / 2,
          Math.min(startTabletX + dx, physW - mapping.tabletW / 2),
        ) * 100,
      ) / 100;
    mapping.tabletY =
      Math.round(
        Math.max(
          mapping.tabletH / 2,
          Math.min(startTabletY + dy, physH - mapping.tabletH / 2),
        ) * 100,
      ) / 100;
  }

  function onTabletPointerUp(e: PointerEvent): void {
    isDraggingTablet = false;
    (e.target as HTMLElement).releasePointerCapture(e.pointerId);
  }

  interface HandlerObject {
    input: (e: Event) => void;
    change: (e: Event) => void;
  }

  function makeHandler(
    applyFn: (val: number, input: HTMLInputElement) => void,
  ): HandlerObject {
    return {
      input: (e: Event) => {
        const el = e.currentTarget as HTMLInputElement;
        if (document.activeElement === el) return;
        applyFn(parseFloat(el.value), el);
      },
      change: (e: Event) => {
        const el = e.currentTarget as HTMLInputElement;
        applyFn(parseFloat(el.value), el);
      },
    };
  }

  const wHandlers = makeHandler((w, input) => {
    if (!Number.isNaN(w)) {
      const clampedW =
        physW > 0 ? Math.max(1.0, Math.min(w, physW)) : Math.max(1.0, w);
      if (mapping.forceAspectRatio && mapping.tabletH > 0) {
        const ratio = mapping.tabletW / mapping.tabletH;
        mapping.tabletW = Math.round(clampedW * 100) / 100;
        mapping.tabletH =
          physH > 0
            ? Math.round(
                Math.max(1.0, Math.min(clampedW / ratio, physH)) * 100,
              ) / 100
            : Math.round(Math.max(1.0, clampedW / ratio) * 100) / 100;
      } else {
        mapping.tabletW = Math.round(clampedW * 100) / 100;
      }
      if (physW > 0)
        mapping.tabletX =
          Math.round(
            Math.max(
              mapping.tabletW / 2,
              Math.min(mapping.tabletX, physW - mapping.tabletW / 2),
            ) * 100,
          ) / 100;
      if (physH > 0)
        mapping.tabletY =
          Math.round(
            Math.max(
              mapping.tabletH / 2,
              Math.min(mapping.tabletY, physH - mapping.tabletH / 2),
            ) * 100,
          ) / 100;
    }
    input.value = String(mapping.tabletW);
  });

  const hHandlers = makeHandler((h, input) => {
    if (!Number.isNaN(h)) {
      const clampedH =
        physH > 0 ? Math.max(1.0, Math.min(h, physH)) : Math.max(1.0, h);
      if (mapping.forceAspectRatio && mapping.tabletH > 0) {
        const ratio = mapping.tabletW / mapping.tabletH;
        mapping.tabletH = Math.round(clampedH * 100) / 100;
        mapping.tabletW =
          physW > 0
            ? Math.round(
                Math.max(1.0, Math.min(clampedH * ratio, physW)) * 100,
              ) / 100
            : Math.round(Math.max(1.0, clampedH * ratio) * 100) / 100;
      } else {
        mapping.tabletH = Math.round(clampedH * 100) / 100;
      }
      if (physW > 0)
        mapping.tabletX =
          Math.round(
            Math.max(
              mapping.tabletW / 2,
              Math.min(mapping.tabletX, physW - mapping.tabletW / 2),
            ) * 100,
          ) / 100;
      if (physH > 0)
        mapping.tabletY =
          Math.round(
            Math.max(
              mapping.tabletH / 2,
              Math.min(mapping.tabletY, physH - mapping.tabletH / 2),
            ) * 100,
          ) / 100;
    }
    input.value = String(mapping.tabletH);
  });

  const xHandlers = makeHandler((x, input) => {
    if (!Number.isNaN(x)) {
      mapping.tabletX =
        physW > 0
          ? Math.round(
              Math.max(
                mapping.tabletW / 2,
                Math.min(x, physW - mapping.tabletW / 2),
              ) * 100,
            ) / 100
          : Math.round(x * 100) / 100;
    }
    input.value = String(mapping.tabletX);
  });

  const yHandlers = makeHandler((y, input) => {
    if (!Number.isNaN(y)) {
      mapping.tabletY =
        physH > 0
          ? Math.round(
              Math.max(
                mapping.tabletH / 2,
                Math.min(y, physH - mapping.tabletH / 2),
              ) * 100,
            ) / 100
          : Math.round(y * 100) / 100;
    }
    input.value = String(mapping.tabletY);
  });

  const rotHandlers = makeHandler((r, input) => {
    if (!Number.isNaN(r)) {
      const clampedR = Math.max(-360, Math.min(r, 360));
      mapping.tabletRotation = Math.round(clampedR * 100) / 100;
    }
    input.value = String(mapping.tabletRotation);
  });
</script>

<div class="section-wrapper">
  <Title>Tablet</Title>

  <div
    bind:this={vizTabletContainer}
    class="visual-area"
    bind:clientWidth={tabletAreaW}
    bind:clientHeight={tabletAreaH}
  >
    <div
      style:width="{tabletCanvasW}px" style:height="{tabletCanvasH}px"
      class="tablet-canvas"
    >
      <div
        style:left="{((mapping.tabletX - mapping.tabletW / 2) / physW) *
          100}%" style:top="{((mapping.tabletY - mapping.tabletH / 2) / physH) * 100}%" style:height="{(mapping.tabletH / physH) * 100}%" style:transform="rotate({mapping.tabletRotation}deg)" style:width="{(mapping.tabletW / physW) * 100}%"
        class="active-area"
        onpointercancel={onTabletPointerUp}
        onpointerdown={onTabletPointerDown}
        onpointermove={onTabletPointerMove}
        onpointerup={onTabletPointerUp}
        role="presentation"
      >
        {#if mapping.showOsuPlayfield}
          <div
            style:width="{((mapping.targetH * OSU_PLAYFIELD_ASPECT) /
              mapping.targetW) *
              100}%" style:height="{(OSU_MENU_INNER_H / mapping.targetH) * 100}%" style:left="{50 -
              ((mapping.targetH * OSU_PLAYFIELD_ASPECT) / mapping.targetW) *
                50}%" style:top="{50 - (OSU_MENU_Y_OFFSET / mapping.targetH) * 100}%"
            class="osu-playfield"
          ></div>
        {/if}
        <div class="ratio-text">
          {mapping.tabletH !== 0
            ? (mapping.tabletW / mapping.tabletH).toFixed(4)
            : "0.0000"}
        </div>
        <div class="dimensions">
          <span class="dim-top">{mapping.tabletW.toFixed(2)}mm</span>
          <span class="dim-left">{mapping.tabletH.toFixed(2)}mm</span>
        </div>
      </div>

      {#if isPenActive}
        <div
          style:left="{(currentPenX / tabletMaxX) * 100}%" style:top="{(currentPenY /
            tabletMaxY) *
            100}%"
          class="pen-cursor"
        ></div>
      {/if}
    </div>
  </div>

  <div class="controls">
    <div
      style:display="flex" style:gap="var(--spacing-md)" style:justify-content="space-between" style:align-items="center"
      class="checkbox-row"
    >
      <div style:display="flex" style:gap="var(--spacing-md)">
        <Checkbox
          label="Force Aspect Ratio"
          bind:checked={mapping.forceAspectRatio}
        />
        <Checkbox
          label="Show Osu!Playfield"
          bind:checked={mapping.showOsuPlayfield}
        />
      </div>
    </div>

    <div class="input-row">
      <Input
        label="Width"
        onchange={wHandlers.change}
        oninput={wHandlers.input}
        precision={2}
        step="0.01"
        type="number"
        unit="mm"
        value={mapping.tabletW}
        width="65px"
      />
      <Input
        label="Height"
        onchange={hHandlers.change}
        oninput={hHandlers.input}
        precision={2}
        step="0.01"
        type="number"
        unit="mm"
        value={mapping.tabletH}
        width="65px"
      />
      <Input
        label="X"
        onchange={xHandlers.change}
        oninput={xHandlers.input}
        precision={2}
        type="number"
        unit="mm"
        value={mapping.tabletX}
        width="65px"
      />
      <Input
        label="Y"
        onchange={yHandlers.change}
        oninput={yHandlers.input}
        precision={2}
        type="number"
        unit="mm"
        value={mapping.tabletY}
        width="65px"
      />
      <Input
        label="Rotation"
        onchange={rotHandlers.change}
        oninput={rotHandlers.input}
        precision={2}
        type="number"
        unit="°"
        value={mapping.tabletRotation}
        width="65px"
      />
    </div>
  </div>
</div>

<style>
  .section-wrapper {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .visual-area {
    background-color: var(--bg-app);
    height: 250px;
    border-radius: var(--radius-sm);
    display: flex;
    align-items: center;
    justify-content: center;
    position: relative;
    overflow: hidden;
    border: var(--border-width) solid var(--border);
  }
  
  .tablet-canvas {
    position: relative;
    background-color: var(--bg-panel);
    border: var(--border-width) solid var(--border);
  }

  .active-area {
    position: absolute;
    background-color: var(--area-tablet);
    outline: var(--border-width) solid var(--area-border-main);
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-active);
    cursor: grab;
    touch-action: none;
    user-select: none;
  }

  .active-area:active {
    cursor: grabbing;
  }

  /* Center dot for the active area - Tablet Area */
  .active-area::after {
    content: "" ;
    position: absolute;
    top: 50%;
    left: 50%;
    width: 4px;
    height: 4px;
    transform: translate(-50%, -50%);
    border-radius: 50%;
    background-color: var(--area-border-main);
    pointer-events: none;
    z-index: var(--z-floating);
  }

  .osu-playfield {
    position: absolute;
    outline: var(--border-width) solid var(--area-border-pink);
    background-color: var(--area-playfield);
    z-index: 1;
    pointer-events: none;
  }

  .ratio-text {
    font-size: var(--font-sm);
    font-weight: var(--font-weight-bold);
    z-index: 2;
    transform: translateY(1.2em);
  }

  .dimensions {
    z-index: 2;
  }

  .dim-top {
    position: absolute;
    top: var(--space-1);
    left: 50%;
    transform: translateX(-50%);
    font-size: var(--font-xs);
  }

  .dim-left {
    position: absolute;
    left: 2px;
    top: 50%;
    transform-origin: top left;
    transform: rotate(-90deg) translateX(-50%);
    font-size: var(--font-xs);
  }

  .pen-cursor {
    position: absolute;
    width: 6px;
    height: 6px;
    background-color: var(--area-contrast);
    border-radius: 50%;
    transform: translate(-50%, -50%);
    pointer-events: none;
    z-index: 100;
  }

  .controls {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    margin-top: var(--space-2);
  }

  .checkbox-row {
    display: flex;
    gap: var(--space-6);
  }

  .input-row {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-4);
  }
</style>
