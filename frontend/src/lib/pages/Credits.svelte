<script lang="ts">
    import { onMount } from "svelte";
    import Title from "../components/ui/Title.svelte";
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
    /** GitHub could not be reached, or refused to answer. */
    let failed = $state(false);
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

                failed = !contribRes.ok && !relRes.ok && !repoRes.ok;

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
                failed = true;
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

<div class="credits">
    <div class="page-header">
        <h1 class="page-title">Credits</h1>
        <p class="page-subtitle">The people behind NextTabletDriver.</p>
    </div>

    <div class="panels">
        <div class="panel">
            {#if loading}
                <p class="status">Loading…</p>
            {:else if failed}
                <p class="status">Could not reach GitHub. Check your connection and open this page again.</p>
            {:else}
                <p class="facts">
                    <b>{formatNumber(totalDownloads)}</b> downloads
                    <span class="dot" aria-hidden="true">·</span>
                    <b>{formatNumber(totalStars)}</b> stars
                    <span class="dot" aria-hidden="true">·</span>
                    <b>{formatNumber(totalReleases)}</b> {totalReleases === 1 ? "release" : "releases"}
                </p>
            {/if}
        </div>

        {#if !loading && !failed}
            <div class="panel">
                <Title>Contributors</Title>
                <div class="list">
                    {#if owner !== undefined}
                        <ContributorRow contributor={owner} isMaintainer={true} />
                    {/if}
                    {#each others as c (c.login)}
                        <ContributorRow contributor={c} />
                    {/each}
                </div>
            </div>
        {/if}
    </div>
</div>

<style>
    /* The same frame as the Release page: title and subtitle on the left, panels under them. */
    .credits {
        display: flex;
        flex-direction: column;
        height: 100%;
        padding: var(--space-5) 4%;
        overflow-y: auto;
        background-color: var(--bg-app);
        color: var(--text-main);
    }

    .page-header {
        margin-bottom: var(--space-8);
    }

    .page-title {
        margin: 0 0 var(--space-2) 0;
        color: var(--text-main);
        font-size: var(--font-5xl);
        font-weight: var(--font-weight-heavy);
    }

    .page-subtitle {
        margin: 0;
        color: var(--text-muted);
        font-size: var(--font-xl);
    }

    .panels {
        display: flex;
        flex-direction: column;
        gap: var(--space-4);
        max-width: 720px;
    }

    .panel {
        display: flex;
        flex-direction: column;
        gap: var(--space-4);
        padding: var(--space-6);
        background-color: var(--bg-panel);
        border: var(--border-width) solid var(--border);
        border-radius: var(--radius-md);
    }

    .status {
        margin: 0;
        color: var(--text-muted);
        font-size: var(--font-lg);
    }

    .facts {
        margin: 0;
        color: var(--text-muted);
        font-size: var(--font-xl);
    }

    .facts b {
        color: var(--text-active);
        font-variant-numeric: tabular-nums;
        font-weight: var(--font-weight-bold);
    }

    .dot {
        margin: 0 var(--space-2);
    }

    .list {
        display: flex;
        flex-direction: column;
        margin: 0 calc(var(--space-3) * -1);
    }
</style>
