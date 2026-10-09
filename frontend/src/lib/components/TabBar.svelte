<script lang="ts">
  let { activeTab = $bindable("output") } = $props();
  
  const tabs = [
      { id: "output", label: "Output" },
      { id: "filters", label: "Filters" },
      { id: "penSettings", label: "Pen Settings" },
      { id: "console", label: "Console" },
      { id: "settings", label: "Settings" },
      { id: "release", label: "Release" },
      { id: "credits", label: "Credits" }
  ];
</script>

<div class="tabs">
  {#each tabs as tab (tab.id)}
      <button 
          class="tab" 
          class:active={activeTab === tab.id}
          onclick={() => { activeTab = tab.id; }}
      >
          {tab.label}
      </button>
  {/each}
</div>

<style>
  .tabs {
      display: flex;
      background: var(--bg-app);
      padding: var(--space-1) var(--space-1);
      gap: var(--space-1);
      flex-shrink: 0;
  }
  
  .tab {
      background: var(--bg-tab);
      border: none;
      color: var(--text-main);
      padding: var(--space-1-5) var(--space-3-5);
      font-size: var(--font-md);
      border-radius: var(--radius-md);
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
