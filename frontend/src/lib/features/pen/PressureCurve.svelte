<script lang="ts">
    import type { MappingState } from "../../features/mapping/mapping-state.svelte";
    import Button from "../../components/ui/Button.svelte";
    import { LivePen } from "./live-pen.svelte";
    import {
        DEFAULT_POINTS, EXPONENT_RANGE, addPoint, evaluateCurve, movePoint, removePoint, sampleCurve,
        type CurvePoint, type CurveType,
    } from "./pressure-curve";

    const { mapping, live = new LivePen() }: {
        mapping: MappingState;
        /** Where the pen is right now. Replaced in tests. */
        live?: LivePen;
    } = $props();

    /** The space around the graph, for the tick labels, in pixels. */
    const PAD = 38;
    const TYPES: CurveType[] = ["Linear", "Exponential", "Custom"];
    const TICKS = [0, 0.5, 1];
    const STEP = 0.01;

    let graph = $state<SVGSVGElement | undefined>(undefined);
    let dragging = $state<number | null>(null);
    /** The side of the graph in pixels. It follows the height of the area, so the text keeps its size. */
    let size = $state(240);

    const span = $derived(size - 2 * PAD);
    const curve = $derived(mapping.config?.pressure_curve);
    const editable = $derived(curve?.curve_type === "Custom");
    const line = $derived(
        curve === undefined
            ? ""
            : sampleCurve(curve).map(([x, y], i) => `${i === 0 ? "M" : "L"}${String(toX(x))},${String(toY(y))}`).join(" "),
    );

    /** The pressure the pen reports for the pressure it is under right now. */
    const output = $derived(curve === undefined ? 0 : evaluateCurve(live.ratio, curve));

    $effect(() => {
        void live.start();
        return () => { live.stop(); };
    });

    $effect(() => {
        const element = graph;
        if (element === undefined || typeof ResizeObserver === "undefined") return;
        const observer = new ResizeObserver((entries) => {
            const width = Math.round(entries.at(0)?.contentRect.width ?? 0);
            if (width > 2 * PAD) size = width;
        });
        observer.observe(element);
        return () => { observer.disconnect(); };
    });

    function toX(x: number): number {
        return PAD + x * span;
    }

    function toY(y: number): number {
        return size - PAD - y * span;
    }

    /** The curve position (0 to 1 on both axes) under a pointer. */
    function fromPointer(event: PointerEvent): CurvePoint {
        const box = graph?.getBoundingClientRect();
        if (box === undefined || box.width === 0) return [0, 0];
        const x = ((event.clientX - box.left) / box.width) * size;
        const y = ((event.clientY - box.top) / box.height) * size;
        return [(x - PAD) / span, (size - PAD - y) / span];
    }

    function commit(points: CurvePoint[]): void {
        if (mapping.config === null) return;
        mapping.config.pressure_curve.points = points;
        mapping.markDirty();
    }

    function select(type: CurveType): void {
        if (mapping.config === null) return;
        mapping.config.pressure_curve.curve_type = type;
        mapping.markDirty();
    }

    function setExponent(value: number): void {
        if (mapping.config === null || Number.isNaN(value)) return;
        mapping.config.pressure_curve.exponent = value;
        mapping.markDirty();
    }

    function startDrag(event: PointerEvent, index: number): void {
        if (!editable) return;
        event.stopPropagation();
        (event.currentTarget as Element).setPointerCapture(event.pointerId);
        dragging = index;
    }

    function drag(event: PointerEvent): void {
        if (dragging === null || curve === undefined) return;
        const [x, y] = fromPointer(event);
        commit(movePoint(curve.points, dragging, x, y));
    }

    function endDrag(): void {
        dragging = null;
    }

    function addAt(event: PointerEvent): void {
        if (!editable || curve === undefined) return;
        const [x, y] = fromPointer(event);
        const next = addPoint(curve.points, x, y);
        if (next !== curve.points) commit(next);
    }

    function remove(index: number): void {
        if (!editable || curve === undefined) return;
        commit(removePoint(curve.points, index));
    }

    function arrowMove(key: string): [number, number] | null {
        switch (key) {
            case "ArrowLeft": return [-STEP, 0];
            case "ArrowRight": return [STEP, 0];
            case "ArrowUp": return [0, STEP];
            case "ArrowDown": return [0, -STEP];
            default: return null;
        }
    }

    function nudge(event: KeyboardEvent, index: number): void {
        const point = curve?.points.at(index);
        if (!editable || curve === undefined || point === undefined) return;
        const move = arrowMove(event.key);
        if (move !== null) {
            event.preventDefault();
            commit(movePoint(curve.points, index, point[0] + move[0], point[1] + move[1]));
        } else if (event.key === "Delete" || event.key === "Backspace") {
            event.preventDefault();
            remove(index);
        }
    }

    function percent(value: number): string {
        return `${String(Math.round(value * 100))}%`;
    }

    function penState(): string {
        if (!live.present) return "Away";
        return live.touching ? "Touching" : "Hovering";
    }
</script>

<div class="visual-area">
    <svg
        bind:this={graph}
        class="graph"
        class:editable
        aria-label="Pressure curve"
        onpointerdown={addAt}
        onpointermove={drag}
        onpointerup={endDrag}
        role="application"
        viewBox="0 0 {size} {size}"
    >
        {#each TICKS as tick (tick)}
            <line class="grid" x1={toX(tick)} x2={toX(tick)} y1={toY(0)} y2={toY(1)} />
            <line class="grid" x1={toX(0)} x2={toX(1)} y1={toY(tick)} y2={toY(tick)} />
            <text class="tick" x={toX(tick)} y={size - 10}>{percent(tick)}</text>
            <text class="tick tick-y" x={PAD - 8} y={toY(tick) + 3}>{percent(tick)}</text>
        {/each}
        <line class="reference" x1={toX(0)} x2={toX(1)} y1={toY(0)} y2={toY(1)} />
        <path class="curve-line" d={line} data-testid="curve-line" />

        {#if live.present && curve !== undefined}
            <g class="live" aria-hidden="true" data-testid="live-dot">
                <line class="guide" x1={toX(live.ratio)} x2={toX(live.ratio)} y1={toY(output)} y2={toY(0)} />
                <line class="guide" x1={toX(0)} x2={toX(live.ratio)} y1={toY(output)} y2={toY(output)} />
                <circle class="dot" class:touching={live.touching} cx={toX(live.ratio)} cy={toY(output)} r="4.5" />
            </g>
        {/if}

        {#if curve !== undefined && editable}
            {#each curve.points as point, index (index)}
                <g
                    class="handle"
                    class:active={dragging === index}
                    aria-label="Curve point {String(index + 1)}"
                    aria-orientation="vertical"
                    aria-valuemax="100"
                    aria-valuemin="0"
                    aria-valuenow={Math.round(point[1] * 100)}
                    aria-valuetext="Raw {percent(point[0])}, reported {percent(point[1])}"
                    ondblclick={() => { remove(index); }}
                    onkeydown={(event) => { nudge(event, index); }}
                    onpointerdown={(event) => { startDrag(event, index); }}
                    role="slider"
                    tabindex="0"
                >
                    <circle class="hit" cx={toX(point[0])} cy={toY(point[1])} r="12" />
                    <rect class="knob" height="9" width="9" x={toX(point[0]) - 4.5} y={toY(point[1]) - 4.5} />
                </g>
            {/each}
        {/if}
    </svg>

    <p class="readout">
        <span>Raw <b data-testid="live-raw">{live.present ? percent(live.ratio) : "—"}</b></span>
        <span>Reported <b data-testid="live-output">{live.present ? percent(output) : "—"}</b></span>
    </p>
    <p class="state">{penState()}</p>
</div>

{#if mapping.config !== null && curve !== undefined}
    <div class="controls">
        <div class="segmented" aria-label="Curve type" role="group">
            {#each TYPES as type (type)}
                <Button onclick={() => { select(type); }} variant={curve.curve_type === type ? 'primary' : 'ghost'}>{type}</Button>
            {/each}
        </div>

        {#if curve.curve_type === "Exponential"}
            <label class="exponent">
                <span>Exponent</span>
                <input
                    max={EXPONENT_RANGE.max}
                    min={EXPONENT_RANGE.min}
                    oninput={(event) => { setExponent(event.currentTarget.valueAsNumber); }}
                    step="0.05"
                    type="range"
                    value={curve.exponent}
                />
                <output>{curve.exponent.toFixed(2)}</output>
            </label>
            <span class="hint">Above 1 the pen feels firmer, below 1 softer.</span>
        {:else if editable}
            <Button onclick={() => { commit(DEFAULT_POINTS.map((p): CurvePoint => [...p])); }} variant="secondary">Reset curve</Button>
            <span class="hint">Drag a point to move it, click the graph to add one, double-click a point to remove it.</span>
        {:else}
            <span class="hint">The pen reports the pressure as it is.</span>
        {/if}
    </div>
{/if}

<style>
    .visual-area {
        position: relative;
        display: flex;
        align-items: center;
        justify-content: center;
        height: 290px;
        padding: var(--space-2);
        background-color: var(--bg-app);
        border: var(--border-width) solid var(--border);
        border-radius: var(--radius-sm);
        overflow: hidden;
    }

    .graph {
        height: 100%;
        aspect-ratio: 1;
        touch-action: none;
        user-select: none;
    }

    .graph.editable {
        cursor: crosshair;
    }

    .grid {
        stroke: var(--border);
        stroke-width: 1;
    }

    .tick {
        fill: var(--text-muted);
        font-size: var(--font-xs);
        font-variant-numeric: tabular-nums;
        text-anchor: middle;
    }

    .tick-y {
        text-anchor: end;
    }

    .reference {
        stroke: var(--text-muted);
        stroke-width: 1;
        stroke-dasharray: 2 4;
        opacity: 0.6;
    }

    .curve-line {
        fill: none;
        stroke: var(--area-display);
        stroke-width: 2.5;
        stroke-linejoin: round;
    }

    .guide {
        stroke: var(--text-muted);
        stroke-width: 1;
        stroke-dasharray: 2 3;
    }

    /* Hollow while the pen hovers, solid once the tip touches. */
    .dot {
        fill: var(--bg-app);
        stroke: var(--area-contrast);
        stroke-width: 2;
    }

    .dot.touching {
        fill: var(--area-contrast);
    }

    .handle {
        cursor: grab;
        outline: none;
    }

    .handle.active {
        cursor: grabbing;
    }

    .hit {
        fill: transparent;
    }

    .knob {
        fill: var(--bg-app);
        stroke: var(--area-display);
        stroke-width: 2;
        transition: fill var(--transition-normal) var(--ease-emphasized);
    }

    .handle:hover .knob,
    .handle.active .knob {
        fill: var(--area-display);
    }

    .handle:focus-visible .knob {
        stroke: var(--area-contrast);
    }

    /* The figures sit in the corners of the area, like the pixel sizes of the Output boxes. */
    .readout,
    .state {
        position: absolute;
        top: var(--space-2);
        margin: 0;
        color: var(--text-muted);
        font-size: var(--font-sm);
        font-variant-numeric: tabular-nums;
    }

    .readout {
        left: var(--space-3);
        display: flex;
        gap: var(--space-4);
    }

    .readout b {
        color: var(--text-active);
        font-weight: var(--font-weight-medium);
    }

    .state {
        right: var(--space-3);
    }

    .controls {
        display: flex;
        flex-wrap: wrap;
        gap: var(--space-2) var(--space-4);
        align-items: center;
    }

    .segmented {
        display: inline-flex;
        overflow: hidden;
        border: var(--border-width) solid var(--border);
        border-radius: var(--radius-md);
    }

    .segmented :global(.btn) {
        border-radius: 0;
    }

    .exponent {
        display: flex;
        gap: var(--space-2);
        align-items: center;
        color: var(--text-main);
        font-size: var(--font-md);
    }

    .exponent input {
        width: 180px;
        accent-color: var(--accent);
    }

    .exponent output {
        min-width: 3ch;
        color: var(--text-active);
        font-variant-numeric: tabular-nums;
    }

    .hint {
        color: var(--text-muted);
        font-size: var(--font-base);
    }
</style>
