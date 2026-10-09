<script lang="ts">
    import TitleBar from "../components/TitleBar.svelte";
</script>

<div class="detached-window">
    <TitleBar title="NextTabletDriver - Input Lag Analysis" />
    <div class="content">
        <h2>Input Lag Analysis</h2>
        <!-- TODO (CRITICAL): This window spawning is broken. Fix it before v2.0.0 release. -->
        <p>This page is running in a detached native Tauri window!</p>
        <p>Real-time performance metrics will be rendered here soon...</p>
    </div>
</div>

<style>
    .detached-window {
        display: flex;
        flex-direction: column;
        height: 100vh;
        width: 100vw;
        background-color: var(--bg-app);
        color: var(--text-main);
        overflow: hidden;
        border: var(--border-width) solid var(--border);
        border-radius: var(--radius-lg); /* To match main window if transparent */
    }
    .content {
        padding: var(--space-5);
        flex: 1;
        overflow-y: auto;
    }
</style>
