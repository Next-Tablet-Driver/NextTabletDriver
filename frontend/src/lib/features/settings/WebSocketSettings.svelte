<script lang="ts">
    import { Wifi } from "lucide-svelte";
    import type { MappingState } from "../../features/mapping/mapping-state.svelte";
    import Checkbox from "../../components/ui/Checkbox.svelte";
    import Input from "../../components/ui/Input.svelte";

    const { mapping }: { mapping: MappingState } = $props();

    function updateConfig(): void {
        mapping.markDirty();
    }
</script>

<div class="section">
    <div class="section-header">
        <Wifi size={20} />
        <h2>WebSocket Server</h2>
    </div>
    
    {#if mapping.config}
    <div class="flex-column settings-group ws-group">
        <div class="ws-header">
            <Checkbox label="Enable WebSocket Server" onchange={updateConfig} bind:checked={mapping.config.websocket.enabled} />
            <span class="badge" class:running={mapping.config.websocket.enabled}>
                {mapping.config.websocket.enabled ? 'RUNNING' : 'STOPPED'}
            </span>
        </div>
        
        <div class="flex-row">
            <div class="input-group">
                <Input label="Port" oninput={updateConfig} type="number" width="80px" bind:value={mapping.config.websocket.port} />
            </div>
            <div class="input-group">
                <Input label="Rate" oninput={updateConfig} type="number" unit="Hz" width="60px" bind:value={mapping.config.websocket.polling_rate_hz} />
            </div>
        </div>
        
        <div class="payload-section">
            <div class="muted payload-label">Payload Data</div>
            <div class="flex-row payload-checkboxes">
                <Checkbox label="Coords" onchange={updateConfig} bind:checked={mapping.config.websocket.send_coordinates} />
                <Checkbox label="Pressure" onchange={updateConfig} bind:checked={mapping.config.websocket.send_pressure} />
                <Checkbox label="Tilt" onchange={updateConfig} bind:checked={mapping.config.websocket.send_tilt} />
                <Checkbox label="Status" onchange={updateConfig} bind:checked={mapping.config.websocket.send_status} />
            </div>
        </div>
    </div>
    {/if}
</div>

<style>
    .section-header {
        display: flex;
        align-items: center;
        gap: var(--space-2);
        margin-bottom: var(--space-6);
        color: var(--text-active);
    }
    
    .section-header h2 {
        font-size: var(--font-xl);
        font-weight: var(--font-weight-bold);
        margin: 0;
    }

    .settings-group {
        padding-left: var(--space-7);
        gap: var(--space-4);
    }
    
    .flex-column {
        display: flex;
        flex-direction: column;
    }

    .flex-row {
        display: flex;
        gap: var(--space-8);
        align-items: center;
    }
    
    .input-group {
        display: flex;
        align-items: center;
        gap: var(--space-4);
    }

    .ws-group {
        gap: var(--space-5);
    }

    .ws-header {
        display: flex;
        align-items: center;
        justify-content: space-between;
        max-width: 400px;
    }

    .badge {
        font-size: var(--font-sm);
        font-weight: var(--font-weight-bold);
        padding: var(--space-0-5) var(--space-2);
        border-radius: var(--radius-md);
        border: var(--border-width) solid var(--error-border);
        color: var(--error);
    }

    .badge.running {
        border-color: var(--success-border);
        color: var(--success);
    }

    .payload-section {
        display: flex;
        flex-direction: column;
        gap: var(--space-2);
        margin-top: var(--space-2);
    }

    .payload-label {
        font-size: var(--font-base);
        color: var(--text-muted);
    }

    .payload-checkboxes {
        gap: var(--space-4);
    }
</style>
