<script lang="ts">
  import Select from './ui/Select.svelte';

  interface Props {
    tabletName?: string;
    profileName?: string;
    version?: string;
    isDirty?: boolean;
    activeMode?: string;
  }

  let { 
    tabletName = "No Tablet Detected", 
    profileName = "Default", 
    version = "Unknown", 
    isDirty = false,
    activeMode = $bindable("Absolute")
  }: Props = $props();

  const modes = [
      { value: "Absolute", label: "Absolute Mode" },
      { value: "Relative", label: "Relative Mode" }
  ];
</script>

<div class="footer">
  <div class="left">
      <Select options={modes} bind:value={activeMode} />
  </div>
  
  <div class="right">
      <Select options={[{ value: tabletName, label: tabletName }]} value={tabletName} />
      <span class="profile" class:dirty={isDirty}>Profile: <strong style:color="var(--text-emphasis)" style:font-style="{isDirty ? 'italic' : 'normal'}">{profileName}{isDirty ? '*' : ''}</strong></span>
      <span class="version">{version}</span>
  </div>
</div>

<style>
  .footer {
      height: 40px;
      background: var(--bg-app);
      display: flex;
      align-items: center;
      justify-content: space-between;
      padding: 0 var(--space-3);
      flex-shrink: 0;
      font-size: var(--font-md);
      color: var(--text-main);
      border-top: var(--border-width) solid var(--border);
  }
  
  .right {
      display: flex;
      align-items: center;
      gap: var(--space-3);
  }
</style>
