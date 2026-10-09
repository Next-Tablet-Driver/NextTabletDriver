<script lang="ts">
    const { icon: Icon, value, name } = $props();
</script>

<div class="stat-card">
    <Icon class="stat-icon" size={22} />
    <div class="stat-info">
        <span class="stat-value">{value}</span>
        <span class="stat-name">{name}</span>
    </div>
</div>

<style>
    .stat-card {
        flex: 1;
        display: flex;
        align-items: center;
        gap: var(--space-4);
        background: var(--shade-20);
        border: var(--border-width) solid var(--overlay-5);
        border-radius: var(--radius-xl);
        padding: var(--space-4) var(--space-5);
        transition: background-color var(--transition-normal) ease;
    }

    .stat-card:hover {
        background: var(--overlay-3);
    }

    .stat-info {
        display: flex;
        flex-direction: column;
        gap: var(--space-0-5);
    }

    .stat-value {
        font-size: var(--font-xl-plus);
        font-weight: var(--font-weight-bold);
        color: var(--text-emphasis);
        line-height: 1;
    }

    .stat-name {
        font-size: var(--font-base);
        color: var(--text-muted);
        text-transform: uppercase;
        letter-spacing: 0.5px;
        font-weight: var(--font-weight-bold);
    }
</style>
