<script lang="ts">
    import type { LogEntry } from "../../infrastructure/tauri/commands";

    interface Props {
        filteredLogs: LogEntry[];
    }
    
    const { filteredLogs }: Props = $props();
    let logsContainer = $state<HTMLElement>();

    function getLevelColor(level: string): string {
        const lvl = level.toUpperCase();
        if (lvl === "INFO") return "var(--info)";
        if (lvl === "WARN") return "var(--warning)";
        if (lvl === "ERROR") return "var(--error)";
        return "var(--debug)";
    }
</script>

<div class="table-container">
    <table class="logs-table">
        <thead>
            <tr>
                <th class="col-time">Time</th>
                <th class="col-level">Level</th>
                <th class="col-group">Group</th>
                <th class="col-msg">Message</th>
            </tr>
        </thead>
    </table>
    <div bind:this={logsContainer} class="table-body-scroll">
        <table class="logs-table body-table">
            <tbody>
                {#each filteredLogs as log (log.time + log.message)}
                    <tr>
                        <td class="col-time">{log.time}</td>
                        <td style:color="{getLevelColor(log.level)}" class="col-level">{log.level}</td>
                        <td class="col-group">{log.group}</td>
                        <td class="col-msg">{log.message}</td>
                    </tr>
                {/each}
                {#if filteredLogs.length === 0}
                    <tr class="empty-row">
                        <td colspan="4">No logs to display</td>
                    </tr>
                {/if}
            </tbody>
        </table>
    </div>
</div>

<style>
    .table-container {
        flex-grow: 1;
        display: flex;
        flex-direction: column;
        overflow: hidden;
    }

    .logs-table {
        width: 100%;
        border-collapse: collapse;
        table-layout: fixed;
    }

    .logs-table th {
        text-align: left;
        padding: var(--space-2-5) var(--space-4);
        font-weight: var(--font-weight-bold);
        font-size: var(--font-sm);
        text-transform: uppercase;
        color: var(--text-muted);
        border-bottom: var(--border-width) solid var(--overlay-5);
        background-color: var(--shade-15);
    }

    .table-body-scroll {
        flex-grow: 1;
        overflow-y: auto;
    }

    .body-table td {
        padding: var(--space-2) var(--space-4);
        font-size: var(--font-md);
        vertical-align: top;
        word-wrap: break-word;
        border-bottom: var(--border-width) solid var(--overlay-3);
    }

    .body-table tr:hover {
        background-color: var(--overlay-4);
    }

    .empty-row td {
        text-align: center;
        padding: var(--space-10);
        color: var(--text-muted);
        font-family: inherit;
    }

    .col-time { width: 110px; color: var(--text-muted); }
    .col-level { width: 80px; font-weight: var(--font-weight-medium); }
    .col-group { width: 150px; font-weight: var(--font-weight-bold); color: var(--overlay-90); }
    .col-msg { width: auto; color: var(--console-text); }
</style>
