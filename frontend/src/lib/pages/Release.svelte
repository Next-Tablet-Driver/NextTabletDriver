<script lang="ts">
    import { invoke } from "@tauri-apps/api/core";
    import { onMount } from "svelte";
    import ReleaseCard from "../features/release/ReleaseCard.svelte";

    interface Release {
        tag_name: string;
        name: string | null;
        body: string | null;
        published_at: string | null;
    }

    let releases = $state<Release[]>([]);
    let loading = $state(true);
    let error = $state("");

    onMount(() => {
        void (async (): Promise<void> => {
            try {
                releases = await invoke<Release[]>("get_releases");
            } catch (e) {
                error = e as string;
            } finally {
                loading = false;
            }
        })();
    });
</script>

<div class="release-container">
    <div class="page-header">
        <h1 class="page-title">Release Notes</h1>
        <p class="page-subtitle">Discover what's new and improved in the driver.</p>
    </div>

    {#if loading}
        <div class="loading-state">
            <div class="spinner"></div>
            <span>Fetching releases from GitHub...</span>
        </div>
    {:else if error !== ""}
        <div class="error-state">
            Failed to load releases: {error}
        </div>
    {:else}
        <div class="releases-list">
            {#each releases as release, index (release.tag_name)}
                <ReleaseCard isLatest={index === 0} {release} />
            {/each}
        </div>
    {/if}
</div>

<style>
    .release-container {
        display: flex;
        flex-direction: column;
        height: 100%;
        background-color: var(--bg-app);
        color: var(--text-main);
        overflow-y: auto;
        padding: var(--space-5) 4%;
    }

    @media (max-width: 768px) {
        .release-container {
            padding: var(--space-6) var(--space-5);
        }
    }

    .page-header {
        margin-bottom: var(--space-12);
        text-align: left;
    }

    .page-title {
        font-size: var(--font-5xl);
        font-weight: var(--font-weight-heavy);
        margin: 0 0 var(--space-2) 0;
        color: var(--text-main);
        letter-spacing: -0.5px;
    }

    .page-subtitle {
        font-size: var(--font-xl);
        color: var(--text-muted);
        margin: 0;
    }

    .loading-state, .error-state {
        display: flex;
        flex-direction: column;
        align-items: center;
        justify-content: center;
        padding: var(--space-20) 0;
        color: var(--text-muted);
        gap: var(--space-4);
    }

    .spinner {
        width: 24px;
        height: 24px;
        border: 2px solid var(--border);
        border-top-color: var(--accent);
        border-radius: 50%;
        animation: spin 1s linear infinite;
    }

    @keyframes spin {
        to { transform: rotate(360deg); }
    }
    
    .error-state {
        color: var(--danger);
    }

    .releases-list {
        display: flex;
        flex-direction: column;
        gap: var(--space-8);
        max-width: 1000px;
        width: 100%;
    }
</style>
