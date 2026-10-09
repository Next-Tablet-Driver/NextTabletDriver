<script lang="ts">
    import { Search, X } from "lucide-svelte";
    
    interface FiltersType {
        Info: boolean;
        Warn: boolean;
        Error: boolean;
        Debug: boolean;
    }
    
    interface Props {
        searchQuery?: string;
        filters: FiltersType;
        toggleFilter: (f: keyof FiltersType) => void;
    }
    
    let { searchQuery = $bindable(""), filters, toggleFilter }: Props = $props();
</script>

<div class="console-toolbar">
    <div class="search-box">
        <Search class="search-icon" size={14} />
        <input placeholder="Search logs..." type="text" bind:value={searchQuery} />
        {#if searchQuery.length > 0}
            <button class="clear-search" aria-label="Clear search" onclick={() => { searchQuery = ""; }}>
                <X size={14} />
            </button>
        {/if}
    </div>
    
    <div class="filter-toggles">
        <button 
            class="filter-btn info" 
            class:active={filters.Info} 
            onclick={() => { toggleFilter('Info'); }}
        >Info</button>
        <button 
            class="filter-btn warn" 
            class:active={filters.Warn} 
            onclick={() => { toggleFilter('Warn'); }}
        >Warn</button>
        <button 
            class="filter-btn error" 
            class:active={filters.Error} 
            onclick={() => { toggleFilter('Error'); }}
        >Error</button>
        <button 
            class="filter-btn debug" 
            class:active={filters.Debug} 
            onclick={() => { toggleFilter('Debug'); }}
        >Debug</button>
    </div>
</div>

<style>
    .console-toolbar {
        display: flex;
        align-items: center;
        padding: var(--space-3) var(--space-4);
        gap: var(--space-4);
        border-bottom: var(--border-width) solid var(--overlay-5);
        background-color: var(--shade-10);
    }

    .search-box {
        display: flex;
        align-items: center;
        background-color: var(--shade-20);
        border: var(--border-width) solid var(--overlay-5);
        border-radius: var(--radius-md-plus);
        padding: 0 var(--space-2-5);
        height: 32px;
        width: 260px;
        position: relative;
        transition: all var(--transition-moderate) ease;
    }

    .search-box:focus-within {
        border-color: var(--overlay-20);
        background-color: var(--shade-30);
    }

    .search-box :global(svg) {
        color: var(--text-muted);
        margin-right: var(--space-1-5);
    }

    .search-box input {
        background: transparent;
        border: none;
        color: inherit;
        outline: none;
        width: 100%;
        font-size: var(--font-md);
        font-family: inherit;
        text-align: left;
    }

    .clear-search {
        background: transparent;
        border: none;
        color: var(--text-muted);
        cursor: pointer;
        display: flex;
        align-items: center;
        justify-content: center;
        padding: var(--space-0-5);
        border-radius: var(--radius-md);
        transition: all var(--transition-normal);
    }

    .clear-search:hover {
        background-color: var(--overlay-10);
        color: var(--text-strong);
    }

    .filter-toggles {
        display: flex;
        gap: var(--space-1-5);
    }

    .filter-btn {
        background: var(--overlay-3);
        border: var(--border-width) solid transparent;
        color: var(--text-muted);
        border-radius: var(--radius-md);
        padding: var(--space-1) var(--space-3-5);
        font-size: var(--font-base);
        font-weight: var(--font-weight-medium);
        cursor: pointer;
        transition: all var(--transition-moderate) var(--ease-emphasized);
    }

    .filter-btn:hover {
        background-color: var(--overlay-8);
        color: var(--text-strong);
    }

    .filter-btn.info.active { background-color: var(--info-bg); color: var(--info); border-color: var(--info-border); }
    .filter-btn.warn.active { background-color: var(--warning-bg); color: var(--warning); border-color: var(--warning-border); }
    .filter-btn.error.active { background-color: var(--error-bg); color: var(--error); border-color: var(--error-border); }
    .filter-btn.debug.active { background-color: var(--debug-bg); color: var(--debug); border-color: var(--debug-border); }
</style>
