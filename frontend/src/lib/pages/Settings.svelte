<script lang="ts">
    import type { MappingState } from "../features/mapping/mapping-state.svelte";
    import GeneralSettings from "../features/settings/GeneralSettings.svelte";
    import ThemeSettings from "../features/settings/ThemeSettings.svelte";
    import LanguageSettings from "../features/settings/LanguageSettings.svelte";
    import WebSocketSettings from "../features/settings/WebSocketSettings.svelte";

    const { mapping }: { mapping: MappingState } = $props();
</script>

<div class="page-container">
    {#if mapping.config}
        <GeneralSettings {mapping} />
        <ThemeSettings {mapping} />
        <LanguageSettings {mapping} />
        <WebSocketSettings {mapping} />
    {/if}
</div>

<style>
    .page-container {
        padding: var(--space-8) var(--space-10);
        height: 100%;
        overflow-y: auto;
        display: flex;
        flex-direction: column;
        gap: var(--space-10);
        max-width: 800px;
    }
</style>
