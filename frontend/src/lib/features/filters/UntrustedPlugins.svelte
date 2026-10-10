<script lang="ts">
    import { ShieldAlert } from "lucide-svelte";
    import type { UntrustedPlugin } from "../../infrastructure/tauri/commands";
    import Button from "../../components/ui/Button.svelte";

    const { plugins, onTrust } = $props<{
        plugins: UntrustedPlugin[];
        onTrust: (sha256: string) => void | Promise<void>;
    }>();
</script>

{#if plugins.length > 0}
    <div class="untrusted-banner" role="alert">
        <div class="banner-title">
            <ShieldAlert size={18} />
            <strong>{plugins.length === 1 ? "1 plugin was not loaded" : `${String(plugins.length)} plugins were not loaded`}</strong>
        </div>
        <p>
            These libraries are in the plugins folder but were not installed through NextTabletDriver.
            Plugins run native code with full access to your system, so they stay disabled until you approve them.
        </p>
        <ul>
            {#each plugins as plugin (plugin.file_name)}
                <li>
                    <span class="file">{plugin.file_name}</span>
                    <code class="hash" title={plugin.sha256}>{plugin.sha256.slice(0, 12)}…</code>
                    <Button onclick={() => onTrust(plugin.sha256)}>Review and trust</Button>
                </li>
            {/each}
        </ul>
    </div>
{/if}

<style>
    .untrusted-banner {
        border: var(--border-width) solid var(--warning);
        background: color-mix(in srgb, var(--warning) 10%, transparent);
        border-radius: var(--radius-md);
        padding: var(--space-4);
        margin-bottom: var(--space-6);
        font-size: var(--font-md);
    }

    .banner-title {
        display: flex;
        align-items: center;
        gap: var(--space-2);
        color: var(--text-active);
    }

    p {
        margin: var(--space-2) 0;
        color: var(--text-muted);
    }

    ul {
        list-style: none;
        margin: 0;
        padding: 0;
        display: flex;
        flex-direction: column;
        gap: var(--space-2);
    }

    li {
        display: flex;
        align-items: center;
        gap: var(--space-4);
    }

    .file {
        flex: 1;
        font-family: var(--font-mono);
    }

    .hash {
        color: var(--text-muted);
        font-family: var(--font-mono);
    }
</style>
