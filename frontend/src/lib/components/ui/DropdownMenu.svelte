<script lang="ts">
  import type { Snippet } from 'svelte';

  type Side = 'top' | 'bottom' | 'left' | 'right';
  type Placement = `${Side}-start`;

  interface Props {
    children?: Snippet;
    placement?: Placement;
    class?: string;
    style?: string;
  }

  const {
    children,
    placement = 'bottom-start',
    class: className = '',
    style = ''
  }: Props = $props();

  const MARGIN = 8; // distance minimale avec le bord de la fenêtre
  const GAP = 8;    // espace entre l'ancre et le menu (axe vertical)

  const OPPOSITE: Record<Side, Side> = {
    top: 'bottom',
    bottom: 'top',
    left: 'right',
    right: 'left'
  };

  let menuRef = $state<HTMLDivElement>();
  let resolvedSide = $state<Side>();
  let alignEnd = $state(false);
  let ready = $state(false);

  const preferredSide = $derived(placement.split('-')[0] as Side);
  const side = $derived(resolvedSide ?? preferredSide);
  const gap = $derived(GAP);

  function getSpace(a: DOMRect, vw: number, vh: number): Record<Side, number> {
    return {
      top: a.top - MARGIN,
      bottom: vh - a.bottom - MARGIN,
      left: a.left - MARGIN,
      right: vw - a.right - MARGIN
    };
  }

  function resolveAlignEnd(a: DOMRect, w: number, h: number, vw: number, vh: number, isVertical: boolean): boolean {
    if (isVertical) {
      return a.left + w > vw - MARGIN && a.right - w >= MARGIN;
    }
    return a.top + h > vh - MARGIN && a.bottom - h >= MARGIN;
  }

  function resolve(): void {
    const menu = menuRef;
    if (menu === undefined) return;
    const anchor = menu.offsetParent as HTMLElement | null; 
    if (anchor === null) return;

    const a = anchor.getBoundingClientRect();
    const w = menu.offsetWidth;
    const h = menu.offsetHeight;
    const vw = window.innerWidth;
    const vh = window.innerHeight;

    const space = getSpace(a, vw, vh);

    const isVertical = preferredSide === 'top' || preferredSide === 'bottom';
    const needed = isVertical ? h + GAP : w;
    const opposite = OPPOSITE[preferredSide];

    resolvedSide = space[preferredSide] < needed && space[opposite] > space[preferredSide] ? opposite : preferredSide;
    alignEnd = resolveAlignEnd(a, w, h, vw, vh, isVertical);
    ready = true;
  }

  $effect(() => {
    if (menuRef === undefined) return;

    resolve();

    const observer = new ResizeObserver(resolve);
    observer.observe(menuRef);
    window.addEventListener('resize', resolve);

    return () => {
      observer.disconnect();
      window.removeEventListener('resize', resolve);
    };
  });
</script>

<div
  bind:this={menuRef}
  style:--gap="{gap}px" {style}
  class="dropdown-menu {className}"
  data-align={alignEnd ? 'end' : 'start'}
  data-ready={ready ? 'true' : 'false'}
  data-side={side}
  role="menu"
>
  {#if children}{@render children()}{/if}
</div>

<style>
  .dropdown-menu {
    position: absolute;
    z-index: var(--menu-zindex);
    display: flex;
    flex-direction: column;
    gap: var(--menu-gap);
    width: max-content;
    min-width: 180px;
    max-width: calc(100vw - 16px);
    padding: var(--menu-padding);
    background-color: var(--menu-bg);
    border: var(--border-width) solid var(--menu-border);
    border-radius: var(--menu-radius-outer);
    visibility: hidden;
  }

  .dropdown-menu[data-ready='true'] {
    visibility: visible;
    animation: menu-in var(--transition-normal) ease-out;
  }

  /* --- Axe principal --- */
  .dropdown-menu[data-side='bottom'] { top: 100%;    margin-top: var(--gap); }
  .dropdown-menu[data-side='top']    { bottom: 100%; margin-bottom: var(--gap); }
  .dropdown-menu[data-side='right']  { left: 100%;   margin-left: var(--gap); z-index: calc(var(--menu-zindex) + 1); }
  .dropdown-menu[data-side='left']   { right: 100%;  margin-right: var(--gap); z-index: calc(var(--menu-zindex) + 1); }

  /* --- Axe transversal --- */
  .dropdown-menu[data-side='bottom'][data-align='start'],
  .dropdown-menu[data-side='top'][data-align='start'] { left: 0; }

  .dropdown-menu[data-side='bottom'][data-align='end'],
  .dropdown-menu[data-side='top'][data-align='end'] { right: 0; }

  /* -5px = padding (4) + bordure (1) : le 1er item s'aligne sur l'item parent */
  .dropdown-menu[data-side='right'][data-align='start'],
  .dropdown-menu[data-side='left'][data-align='start'] { top: -5px; }

  .dropdown-menu[data-side='right'][data-align='end'],
  .dropdown-menu[data-side='left'][data-align='end'] { bottom: -5px; }

  /* --- Origine de l'animation selon le placement --- */
  .dropdown-menu[data-side='bottom'] { transform-origin: top left; }
  .dropdown-menu[data-side='bottom'][data-align='end'] { transform-origin: top right; }
  .dropdown-menu[data-side='top'] { transform-origin: bottom left; }
  .dropdown-menu[data-side='top'][data-align='end'] { transform-origin: bottom right; }
  .dropdown-menu[data-side='right'] { transform-origin: top left; }
  .dropdown-menu[data-side='right'][data-align='end'] { transform-origin: bottom left; }
  .dropdown-menu[data-side='left'] { transform-origin: top right; }
  .dropdown-menu[data-side='left'][data-align='end'] { transform-origin: bottom right; }

  @keyframes menu-in {
    from { opacity: 0; transform: scale(0.96); }
    to   { opacity: 1; transform: scale(1); }
  }

  @media (prefers-reduced-motion: reduce) {
    .dropdown-menu[data-ready='true'] { animation: none; }
  }
</style>