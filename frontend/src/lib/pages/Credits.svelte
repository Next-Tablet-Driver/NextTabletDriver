<script lang="ts">
    import { onMount } from "svelte";
    import { Download, Star, Tag } from "lucide-svelte";
    import StatCard from "../features/credits/StatCard.svelte";
    import ContributorRow from "../features/credits/ContributorRow.svelte";

    interface Contributor {
        login: string;
        avatar_url: string;
        html_url: string;
        contributions: number;
    }

    interface GithubAsset { name: string; download_count: number }
    interface GithubRelease { assets: GithubAsset[] }

    let contributors = $state<Contributor[]>([]);
    let loading = $state(true);
    let totalDownloads = $state(0);
    let totalStars = $state(0);
    let totalReleases = $state(0);

    onMount(() => {
        void (async (): Promise<void> => {
            try {
                const [contribRes, relRes, repoRes] = await Promise.all([
                    fetch("https://api.github.com/repos/Next-Tablet-Driver/NextTabletDriver/contributors"),
                    fetch("https://api.github.com/repos/Next-Tablet-Driver/NextTabletDriver/releases?per_page=100"),
                    fetch("https://api.github.com/repos/Next-Tablet-Driver/NextTabletDriver")
                ]);

                if (contribRes.ok) {
                    contributors = (await contribRes.json()) as Contributor[];
                }
                if (relRes.ok) {
                    const relData = (await relRes.json()) as GithubRelease[];
                    totalReleases = relData.length;
                    totalDownloads = relData.reduce((acc: number, rel: GithubRelease) => 
                        acc + rel.assets
                            // Exclude updater manifests and signatures to avoid double-counting auto-updates
                            .filter((ast: GithubAsset) => !ast.name.endsWith('.json') && !ast.name.endsWith('.sig'))
                            .reduce((a: number, ast: GithubAsset) => a + ast.download_count, 0), 0
                    );
                }
                if (repoRes.ok) {
                    const repoData = (await repoRes.json()) as { stargazers_count: number };
                    totalStars = repoData.stargazers_count;
                }
            } catch (e) {
                console.error("Failed to fetch GitHub stats:", e);
            } finally {
                loading = false;
            }
        })();
    });

    const owner = $derived(contributors.find(c => c.login.toLowerCase() === "isweat-exe"));
    const others = $derived(contributors.filter(c => 
        c.login.toLowerCase() !== "isweat-exe" && 
        !c.login.toLowerCase().includes("bot") &&
        c.login.toLowerCase() !== "dependabot[bot]"
    ).sort((a,b) => b.contributions - a.contributions));

    function formatNumber(num: number): string {
        return new Intl.NumberFormat('en-US').format(num);
    }
</script>

<div class="credits-container">
    <div class="header">
        <h1 class="page-title">Credits</h1>
        <p class="subtitle">A special thanks to everyone who contributed to NextTabletDriver.</p>
    </div>

    {#if loading}
        <div class="loading-state">Fetching stats and contributors...</div>
    {:else}
        <div class="stats-cards">
            <StatCard name="Downloads" icon={Download} value={formatNumber(totalDownloads)} />
            <StatCard name="Stars" icon={Star} value={formatNumber(totalStars)} />
            <StatCard name="Releases" icon={Tag} value={formatNumber(totalReleases)} />
        </div>

        <div class="contributors-list">
            {#if owner !== undefined}
                <ContributorRow contributor={owner} isMaintainer={true} />
            {/if}

            {#each others as c (c.login)}
                <ContributorRow contributor={c} />
            {/each}
        </div>
    {/if}
</div>

<style>
    .credits-container {
        display: flex;
        flex-direction: column;
        align-items: center;
        height: 100%;
        background-color: var(--bg-app);
        color: var(--text-strong);
        font-family: var(--font-sans);
        overflow-y: auto;
        padding: var(--space-14) var(--space-8);
    }

    .header {
        text-align: center;
        margin-bottom: var(--space-12);
        display: flex;
        flex-direction: column;
        align-items: center;
        gap: var(--space-4);
    }

    .page-title {
        font-size: var(--font-4xl);
        font-weight: var(--font-weight-bold);
        margin: 0;
        color: var(--text-emphasis);
        letter-spacing: -0.5px;
    }

    .subtitle {
        font-size: var(--font-lg-plus);
        color: var(--text-muted);
        margin: 0;
    }

    .loading-state {
        text-align: center;
        padding: var(--space-10);
        color: var(--text-muted);
    }

    .stats-cards {
        display: flex;
        justify-content: center;
        gap: var(--space-4);
        width: 100%;
        max-width: 640px;
        margin-bottom: var(--space-8);
    }

    .contributors-list {
        display: flex;
        flex-direction: column;
        width: 100%;
        max-width: 640px;
        background: var(--shade-20);
        border: var(--border-width) solid var(--overlay-5);
        border-radius: var(--radius-xl);
        overflow: hidden;
    }
</style>
