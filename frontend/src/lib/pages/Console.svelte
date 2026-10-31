<script lang="ts">
  import { log } from "../infrastructure/log";
  import { onMount, onDestroy } from "svelte";
  import { getLogs, clearLogs, type LogEntry } from "../infrastructure/tauri/commands";

  import ConsoleToolbar from "../features/console/ConsoleToolbar.svelte";
  import ConsoleTable from "../features/console/ConsoleTable.svelte";
  import ConsoleFooter from "../features/console/ConsoleFooter.svelte";

  let allLogs = $state<LogEntry[]>([]);
  let searchQuery = $state("");
  const filters = $state({
    Info: true,
    Warn: true,
    Error: true,
    Debug: true,
  });

  let intervalId: ReturnType<typeof setInterval>;

  async function fetchLogs(): Promise<void> {
    try {
      const newLogs = await getLogs();
      allLogs = newLogs;
    } catch (e) {
      log.error("Failed to fetch logs:", e);
    }
  }

  onMount(() => {
    void fetchLogs();
    intervalId = setInterval(() => {
      void fetchLogs();
    }, 500);
  });

  onDestroy(() => {
    clearInterval(intervalId);
  });

  const filteredLogs = $derived(
    allLogs.filter((entry) => {
      const lvl = entry.level.toUpperCase();
      if (lvl === "INFO" && !filters.Info) return false;
      if (lvl === "WARN" && !filters.Warn) return false;
      if (lvl === "ERROR" && !filters.Error) return false;
      if (lvl === "DEBUG" && !filters.Debug) return false;

      if (searchQuery.trim().length > 0) {
        return entry.search_text.includes(searchQuery.trim().toLowerCase());
      }
      return true;
    }),
  );

  function toggleFilter(f: keyof typeof filters): void {
    filters[f] = !filters[f];
  }

  async function clearConsole(): Promise<void> {
    try {
      await clearLogs();
      allLogs = [];
    } catch (e) {
      log.error(e);
    }
  }

  async function copyVisible(): Promise<void> {
    const text = filteredLogs
      .map((l) => `[${l.time}] [${l.level}] [${l.group}] ${l.message}`)
      .join("\n");
    try {
      await navigator.clipboard.writeText(text);
    } catch (e) {
      log.error(e);
    }
  }

  function exportAll(): void {
    const text = allLogs
      .map((l) => `[${l.time}] [${l.level}] [${l.group}] ${l.message}`)
      .join("\n");
    const blob = new Blob([text], { type: "text/plain" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = "NextTabletDriver_logs.txt";
    a.click();
    URL.revokeObjectURL(url);
  }
</script>

<div class="console-container">
  <ConsoleToolbar {filters} {toggleFilter} bind:searchQuery />
  <ConsoleTable {filteredLogs} />
  <ConsoleFooter
    {clearConsole}
    {copyVisible}
    {exportAll}
    filteredCount={filteredLogs.length}
    totalCount={allLogs.length}
  />
</div>

<style>
  .console-container {
    display: flex;
    flex-direction: column;
    height: 100%;
    background-color: var(--bg-app);
    color: var(--text-strong);
    font-family: var(--font-sans);
  }
</style>
