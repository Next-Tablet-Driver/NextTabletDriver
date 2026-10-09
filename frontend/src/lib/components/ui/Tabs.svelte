<script lang="ts">
  let { 
      activeTab = $bindable(), 
      tabs = [] 
  }: { 
      activeTab: string,
      tabs: string[]
  } = $props();
</script>

<div class="tabs">
  {#each tabs as tab (tab)}
      <button 
          class="tab" 
          class:active={activeTab === tab} 
          onclick={() => { activeTab = tab; }}
      >
          {tab}
      </button>
  {/each}
</div>

<style>
  .tabs {
      display: flex;
      background: var(--bg-app);
      padding: 0 var(--space-2);
      gap: var(--space-0-5);
      flex-shrink: 0;
  }
  
  .tab {
      background: var(--bg-tab);
      border: none;
      color: var(--text-main);
      padding: var(--space-1-5) var(--space-3-5);
      font-size: var(--font-md);
      border-radius: var(--radius-md) var(--radius-md) 0 0;
      cursor: pointer;
  }
  
  .tab:hover {
      background: var(--bg-panel);
      color: var(--text-active);
  }
  
  .tab.active {
      background: var(--bg-tab-active);
      color: var(--text-on-accent);
  }
</style>
