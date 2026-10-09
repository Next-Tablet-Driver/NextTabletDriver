<script lang="ts">
    import { Activity } from "lucide-svelte";
    import type { MappingState } from "../../features/mapping/mapping-state.svelte";
    import Button from "../../components/ui/Button.svelte";

    const { mapping }: { mapping: MappingState } = $props();

    function updateConfig(): void {
        mapping.markDirty();
    }
</script>

<div class="card curve-card">
    <div class="section-header small">
        <Activity size={18} />
        <h3>Pressure Curve</h3>
    </div>
    
    {#if mapping.config}
        <div class="segmented-control">
            <Button onclick={() => { if (mapping.config !== null) { mapping.config.pressure_curve.curve_type = 'Linear'; updateConfig(); } }} variant={mapping.config.pressure_curve.curve_type === 'Linear' ? 'primary' : 'ghost'}>Linear</Button>
            <Button onclick={() => { if (mapping.config !== null) { mapping.config.pressure_curve.curve_type = 'Exponential'; updateConfig(); } }} variant={mapping.config.pressure_curve.curve_type === 'Exponential' ? 'primary' : 'ghost'}>Exponential</Button>
            <Button onclick={() => { if (mapping.config !== null) { mapping.config.pressure_curve.curve_type = 'Custom'; updateConfig(); } }} variant={mapping.config.pressure_curve.curve_type === 'Custom' ? 'primary' : 'ghost'}>Custom</Button>
        </div>
    {/if}
    
    <div class="pressure-visualizer">
        <div class="bar-container">
            <div class="bar-label">0%</div>
            <div class="bar-bg"><div style:height="0%" class="bar-fill"></div></div>
            <div class="bar-label bottom">Raw</div>
        </div>
        <div class="bar-container">
            <div class="bar-label">0%</div>
            <div class="bar-bg"><div style:height="0%" class="bar-fill"></div></div>
            <div class="bar-label bottom">Curve</div>
        </div>
    </div>
</div>

<style>
    .card {
        display: flex;
        flex-direction: column;
        gap: var(--space-4);
    }
    
    .curve-card {
        grid-column: span 1;
        margin-top: var(--space-6);
    }

    .section-header {
        display: flex;
        align-items: center;
        gap: var(--space-2);
        color: var(--text-active);
    }
    
    .section-header.small {
        margin-bottom: var(--space-4);
    }
    
    .section-header.small h3 {
        font-size: var(--font-lg-plus);
        font-weight: var(--font-weight-bold);
        margin: 0;
    }

    .segmented-control {
        display: inline-flex;
        background: var(--bg-panel);
        padding: var(--space-0-5);
        border-radius: var(--radius-md);
        margin-bottom: var(--space-6);
    }
    .segmented-control :global(.btn) {
        border-radius: calc(var(--radius-md) - var(--radius-sm));
    }
    
    .pressure-visualizer {
        display: flex;
        gap: var(--space-8);
        justify-content: center;
        padding: var(--space-6) 0;
        background: var(--bg-panel);
        border-radius: var(--radius-lg);
        width: 140px;
    }
    
    .bar-container {
        display: flex;
        flex-direction: column;
        align-items: center;
        gap: var(--space-2);
    }
    
    .bar-label {
        font-size: var(--font-sm);
        color: var(--text-muted);
        font-weight: var(--font-weight-bold);
    }
    
    .bar-label.bottom {
        color: var(--text-muted);
    }
    
    .bar-bg {
        width: 16px;
        height: 120px;
        background: var(--bg-app);
        border-radius: var(--radius-lg);
        position: relative;
        overflow: hidden;
    }
    
    .bar-fill {
        position: absolute;
        bottom: 0;
        left: 0;
        width: 100%;
        background: var(--accent);
        transition: height var(--transition-quick);
    }
</style>
