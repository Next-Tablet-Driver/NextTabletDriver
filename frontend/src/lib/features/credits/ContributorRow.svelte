<script lang="ts">
    import { ExternalLink, Link } from "lucide-svelte";

    interface Contributor {
        login: string;
        avatar_url: string;
        html_url: string;
        contributions: number;
    }

    interface Props {
        contributor: Contributor;
        isMaintainer?: boolean;
    }

    const { contributor, isMaintainer = false }: Props = $props();
</script>

<a class="contributor-row" class:maintainer-row={isMaintainer} href={contributor.html_url} rel="noopener noreferrer" target="_blank">
    <div class="c-left">
        <img class="avatar" alt={contributor.login} src={contributor.avatar_url} />
        <div class="c-info">
            <span class="c-name">
                @{contributor.login}
                {#if isMaintainer}
                    <span class="tag tag-maintainer">Maintainer</span>
                {/if}
            </span>
        </div>
    </div>
    <div class="c-right">
        <span class="c-commits">{contributor.contributions} commits</span>
        <span class="link-icon">
            {#if isMaintainer}
                <Link size={18} />
            {:else}
                <ExternalLink size={16} />
            {/if}
        </span>
    </div>
</a>

<style>
    .contributor-row {
        display: flex;
        align-items: center;
        justify-content: space-between;
        padding: var(--space-4) var(--space-6);
        text-decoration: none;
        border-bottom: var(--border-width) solid var(--overlay-3);
        transition: background-color var(--transition-normal) ease;
    }

    .contributor-row:last-child {
        border-bottom: none;
    }

    .contributor-row:hover {
        background-color: var(--overlay-4);
    }

    .maintainer-row {
        background: color-mix(in srgb, var(--info) 3%, transparent);
    }

    .maintainer-row:hover {
        background: color-mix(in srgb, var(--info) 6%, transparent);
    }

    .c-left {
        display: flex;
        align-items: center;
        gap: var(--space-4);
    }

    .avatar {
        width: 40px;
        height: 40px;
        border-radius: 50%;
        object-fit: cover;
        border: var(--border-width) solid var(--overlay-10);
    }

    .c-info {
        display: flex;
        flex-direction: column;
    }

    .c-name {
        display: flex;
        align-items: center;
        gap: var(--space-2-5);
        font-size: var(--font-lg-plus);
        font-weight: var(--font-weight-medium);
        color: var(--text-strong);
        transition: color var(--transition-normal) ease;
    }
    
    .contributor-row:hover .c-name {
        color: var(--text-emphasis);
    }

    .tag {
        font-size: var(--font-xs);
        font-weight: var(--font-weight-bold);
        padding: var(--space-0-5) var(--space-2);
        border-radius: var(--radius-pill);
        letter-spacing: 0.5px;
        text-transform: uppercase;
    }

    .tag-maintainer {
        background-color: var(--info-bg);
        color: var(--info);
        border: var(--border-width) solid color-mix(in srgb, var(--info) 20%, transparent);
    }

    .c-right {
        display: flex;
        align-items: center;
        gap: var(--space-5);
    }

    .c-commits {
        font-size: var(--font-md);
        color: var(--text-muted);
        font-variant-numeric: tabular-nums;
    }

    .link-icon {
        color: var(--text-muted);
        transition: color var(--transition-normal) ease;
        display: flex;
    }

    .contributor-row:hover .link-icon {
        color: var(--text-emphasis);
    }
</style>
