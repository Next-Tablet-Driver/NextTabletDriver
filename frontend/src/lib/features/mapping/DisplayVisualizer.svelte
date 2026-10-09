<script lang="ts">
    import { onMount } from 'svelte';
    import { windowControls, type Monitor } from '../../infrastructure/window';
    import type { MappingState } from './mapping-state.svelte';
    import Title from '../../components/ui/Title.svelte';
    import Checkbox from '../../components/ui/Checkbox.svelte';
    import Input from '../../components/ui/Input.svelte';

    const { mapping }: { mapping: MappingState } = $props();

    let monitors = $state<Monitor[]>([]);
    let minX = $state(0), minY = $state(0), maxX = $state(0), maxY = $state(0);
    const deskW = $derived((maxX - minX) !== 0 ? (maxX - minX) : 1920);
    const deskH = $derived((maxY - minY) !== 0 ? (maxY - minY) : 1080);

    let displayAreaW = $state(0);
    let displayAreaH = $state(0);
    const displayScale = $derived(displayAreaW > 0 && displayAreaH > 0 ? Math.min((displayAreaW * 0.95) / deskW, (displayAreaH * 0.95) / deskH) : 0);
    const canvasW = $derived(deskW * displayScale);
    const canvasH = $derived(deskH * displayScale);

    onMount(() => {
        void (async (): Promise<void> => {
            try {
                const mons = await windowControls.getMonitors();
                if (mons.length > 0) {
                    monitors = mons;
                    minX = Math.min(...mons.map(m => m.position.x));
                    minY = Math.min(...mons.map(m => m.position.y));
                    maxX = Math.max(...mons.map(m => m.position.x + m.size.width));
                    maxY = Math.max(...mons.map(m => m.position.y + m.size.height));
                    
                    if (mapping.targetW === 1920 && mapping.targetH === 1080 && mapping.targetX === 0 && mapping.targetY === 0) {
                        mapping.targetX = minX + (deskW - mapping.targetW) / 2;
                        mapping.targetY = minY + (deskH - mapping.targetH) / 2;
                    }
                } else {
                    monitors = [{ name: 'Fallback', position: {x: 0, y: 0}, size: {width: 1920, height: 1080} } as unknown as Monitor];
                }
            } catch(e) {
                console.warn("Could not load display data", e);
            }
        })();
    });

    let vizDisplayContainer: HTMLDivElement | undefined = $state();
    let isDraggingDisplay = false;
    let startX = 0, startY = 0;
    let startTargetX = 0, startTargetY = 0;

    function onDisplayPointerDown(e: PointerEvent): void {
        isDraggingDisplay = true;
        startX = e.clientX;
        startY = e.clientY;
        startTargetX = mapping.targetX;
        startTargetY = mapping.targetY;
        (e.target as HTMLElement).setPointerCapture(e.pointerId);
    }

    function calculateSnapX(newX: number, snapDist: number): number {
        let bestX = newX;
        let minDistX = Infinity;
        for (const m of monitors) {
            const edgesX = [m.position.x, m.position.x + m.size.width];
            for (const edgeX of edgesX) {
                const distLeft = Math.abs(newX - edgeX);
                if (distLeft < snapDist && distLeft < minDistX) {
                    minDistX = distLeft;
                    bestX = edgeX;
                }
                const distRight = Math.abs((newX + mapping.targetW) - edgeX);
                if (distRight < snapDist && distRight < minDistX) {
                    minDistX = distRight;
                    bestX = edgeX - mapping.targetW;
                }
            }
        }
        return bestX;
    }

    function calculateSnapY(newY: number, snapDist: number): number {
        let bestY = newY;
        let minDistY = Infinity;
        for (const m of monitors) {
            const edgesY = [m.position.y, m.position.y + m.size.height];
            for (const edgeY of edgesY) {
                const distTop = Math.abs(newY - edgeY);
                if (distTop < snapDist && distTop < minDistY) {
                    minDistY = distTop;
                    bestY = edgeY;
                }
                const distBottom = Math.abs((newY + mapping.targetH) - edgeY);
                if (distBottom < snapDist && distBottom < minDistY) {
                    minDistY = distBottom;
                    bestY = edgeY - mapping.targetH;
                }
            }
        }
        return bestY;
    }

    function onDisplayPointerMove(e: PointerEvent): void {
        if (!isDraggingDisplay || vizDisplayContainer === undefined) return;
        
        const canvas = vizDisplayContainer.querySelector('.display-canvas');
        if (canvas === null) return;
        
        const canvasWidth = canvas.getBoundingClientRect().width;
        const scale = deskW / canvasWidth;
        
        const dx = (e.clientX - startX) * scale;
        const dy = (e.clientY - startY) * scale;
        
        let newX = startTargetX + dx;
        let newY = startTargetY + dy;

        if (mapping.edgeSnapping) {
            newX = calculateSnapX(newX, 120.0);
            newY = calculateSnapY(newY, 120.0);
        }

        mapping.targetX = Math.round(Math.max(minX, Math.min(newX, maxX - mapping.targetW)));
        mapping.targetY = Math.round(Math.max(minY, Math.min(newY, maxY - mapping.targetH)));
    }

    function onDisplayPointerUp(e: PointerEvent): void {
        isDraggingDisplay = false;
        (e.target as HTMLElement).releasePointerCapture(e.pointerId);
    }

    interface HandlerObject {
        input: (e: Event) => void;
        change: (e: Event) => void;
    }

    function makeHandler(applyFn: (val: number, input: HTMLInputElement) => void): HandlerObject {
        return {
            input: (e: Event) => {
                const el = e.currentTarget as HTMLInputElement;
                if (document.activeElement === el) return;
                applyFn(parseFloat(el.value), el);
            },
            change: (e: Event) => {
                const el = e.currentTarget as HTMLInputElement;
                applyFn(parseFloat(el.value), el);
            }
        };
    }

    const xHandlers = makeHandler((x, input) => {
        if(!Number.isNaN(x)) {
            const newX = x - mapping.targetW / 2 + minX; 
            mapping.targetX = Math.round(Math.max(minX, Math.min(newX, maxX - mapping.targetW)));
        }
        input.value = String(Math.round(mapping.targetX - minX + mapping.targetW / 2));
    });

    const yHandlers = makeHandler((y, input) => {
        if(!Number.isNaN(y)) {
            const newY = y - mapping.targetH / 2 + minY; 
            mapping.targetY = Math.round(Math.max(minY, Math.min(newY, maxY - mapping.targetH)));
        }
        input.value = String(Math.round(mapping.targetY - minY + mapping.targetH / 2));
    });

    const wHandlers = makeHandler((w, input) => {
        if(!Number.isNaN(w)) {
            mapping.targetW = Math.round(Math.max(1, Math.min(w, deskW)));
            mapping.targetX = Math.round(Math.max(minX, Math.min(mapping.targetX, maxX - mapping.targetW)));
        }
        input.value = String(mapping.targetW);
    });

    const hHandlers = makeHandler((h, input) => {
        if(!Number.isNaN(h)) {
            mapping.targetH = Math.round(Math.max(1, Math.min(h, deskH)));
            mapping.targetY = Math.round(Math.max(minY, Math.min(mapping.targetY, maxY - mapping.targetH)));
        }
        input.value = String(mapping.targetH);
    });
</script>

<div class="section-wrapper">
    <Title>Display</Title>
    
    <div bind:this={vizDisplayContainer} class="visual-area" bind:clientWidth={displayAreaW} bind:clientHeight={displayAreaH}>
    <div style:width="{canvasW}px" style:height="{canvasH}px" class="display-canvas">
        {#each monitors as m (m.name)}
            <div style:left="{((m.position.x - minX) / deskW) * 100}%" style:top="{((m.position.y - minY) / deskH) * 100}%" style:height="{(m.size.height / deskH) * 100}%" style:width="{(m.size.width / deskW) * 100}%" class="monitor-bg">
                {m.size.width}x{m.size.height}
            </div>
        {/each}
        
        <div style:left="{((mapping.targetX - minX) / deskW) * 100}%" style:top="{((mapping.targetY - minY) / deskH) * 100}%" style:height="{(mapping.targetH / deskH) * 100}%" style:width="{(mapping.targetW / deskW) * 100}%" 
             class="target-area"
             onpointercancel={onDisplayPointerUp}
             onpointerdown={onDisplayPointerDown}
             onpointermove={onDisplayPointerMove}
             onpointerup={onDisplayPointerUp}
             role="presentation">
            <div class="ratio-text">{mapping.targetH !== 0 ? (mapping.targetW / mapping.targetH).toFixed(4) : "0.0000"}</div>
            <div class="dimensions">
                <span class="dim-top">{mapping.targetW}px</span>
                <span class="dim-left">{mapping.targetH}px</span>
            </div>
        </div>
    </div>
</div>

<div class="controls">
    <div class="checkbox-row">
        <Checkbox label="Enable Edge Snapping" bind:checked={mapping.edgeSnapping} />
    </div>
    
    <div class="input-row">
        <Input label="Width" onchange={wHandlers.change} oninput={wHandlers.input} type="number" unit="px" value={mapping.targetW} />
        <Input label="Height" onchange={hHandlers.change} oninput={hHandlers.input} type="number" unit="px" value={mapping.targetH} />
        <Input 
            label="X" 
            onchange={xHandlers.change} 
            oninput={xHandlers.input} 
            type="number" unit="px" value={Math.round(mapping.targetX - minX + mapping.targetW / 2)} 
        />
        <Input 
            label="Y" 
            onchange={yHandlers.change} 
            oninput={yHandlers.input} 
            type="number" unit="px" value={Math.round(mapping.targetY - minY + mapping.targetH / 2)} 
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
  
  .display-canvas {
      position: relative;
      background-color: var(--bg-app);
  }
  
  .monitor-bg {
      position: absolute;
      background-color: var(--bg-app);
      border: var(--border-width) solid var(--border);
      color: var(--text-muted);
      display: flex;
      align-items: center;
      justify-content: center;
      font-size: var(--font-sm);
  }
  
  .target-area {
      position: absolute;
      background-color: var(--area-display);
      border: var(--border-width) solid var(--area-contrast);
      color: var(--area-contrast);
      display: flex;
      align-items: center;
      justify-content: center;
      cursor: grab;
      touch-action: none;
      user-select: none;
  }
  .target-area:active { cursor: grabbing; }
  
  .ratio-text {
      font-size: var(--font-sm);
      font-weight: var(--font-weight-bold);
      z-index: 2;
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
