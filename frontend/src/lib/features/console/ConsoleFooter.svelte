<script lang="ts">
    import { Trash2, Copy, Download } from "lucide-svelte";
    import Button from "../../components/ui/Button.svelte";
    
    const { clearConsole, copyVisible, exportAll, filteredCount, totalCount } = $props();
</script>

<div class="console-footer">
    <div class="footer-actions">
        <Button onclick={clearConsole}>
            <Trash2 size={14} /> Clear Console
        </Button>
        <Button onclick={copyVisible}>
            <Copy size={14} /> Copy Visible
        </Button>
        <Button onclick={exportAll}>
            <Download size={14} /> Export All
        </Button>
    </div>
    <div class="footer-stats">
        Showing {filteredCount} / {totalCount} logs
    </div>
</div>

<style>
    .console-footer {
        display: flex;
        justify-content: space-between;
        align-items: center;
        padding: var(--space-2-5) var(--space-4);
        background-color: var(--shade-20);
        border-top: var(--border-width) solid var(--overlay-5);
    }

    .footer-actions {
        display: flex;
        gap: var(--space-2-5);
    }

    .footer-stats {
        font-size: var(--font-base);
        color: var(--text-muted);
    }
</style>
