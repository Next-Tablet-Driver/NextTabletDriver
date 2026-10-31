<script lang="ts">
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

<a class="row" href={contributor.html_url} rel="noopener noreferrer" target="_blank">
    <img class="avatar" alt="" height="28" src={contributor.avatar_url} width="28" />
    <span class="name">@{contributor.login}</span>
    {#if isMaintainer}
        <span class="role">maintainer</span>
    {/if}
    <span class="commits">{contributor.contributions} commits</span>
</a>

<style>
    .row {
        display: flex;
        align-items: center;
        gap: var(--space-3);
        padding: var(--space-2) var(--space-3);
        color: var(--text-main);
        text-decoration: none;
        border-radius: var(--radius-md);
        transition: background-color var(--transition-normal) var(--ease-emphasized);
    }

    .row:hover {
        background: var(--overlay-5);
    }

    .avatar {
        flex-shrink: 0;
        width: 28px;
        height: 28px;
        border-radius: 50%;
        object-fit: cover;
    }

    .name {
        font-size: var(--font-lg);
    }

    .row:hover .name {
        color: var(--text-active);
        text-decoration: underline;
        text-underline-offset: 2px;
    }

    .role {
        color: var(--text-muted);
        font-size: var(--font-md);
    }

    .commits {
        margin-left: auto;
        color: var(--text-muted);
        font-size: var(--font-md);
        font-variant-numeric: tabular-nums;
    }
</style>
