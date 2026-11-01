import { useEffect, useMemo, useState } from "react";
import { ConsoleFooter } from "@/features/console/ConsoleFooter";
import { ConsoleTable } from "@/features/console/ConsoleTable";
import { ConsoleToolbar } from "@/features/console/ConsoleToolbar";
import {
  LEVELS,
  MAX_ROWS,
  downloadText,
  filterLogs,
  formatLogs,
  type Level,
} from "@/features/console/logs";
import { log } from "@/infrastructure/log";
import { clearLogs, getLogs, type LogEntry } from "@/infrastructure/tauri/commands";
import styles from "@/pages/ConsolePage.module.css";

const REFRESH_MS = 500;

/** The application's log, refreshed while the page is open. */
export function ConsolePage() {
  const [entries, setEntries] = useState<LogEntry[]>([]);
  const [shown, setShown] = useState<ReadonlySet<Level>>(new Set(LEVELS));
  const [query, setQuery] = useState("");

  useEffect(() => {
    const refresh = (): void => {
      getLogs().then(setEntries, (e: unknown) => {
        log.error("Failed to fetch logs:", e);
      });
    };
    refresh();
    const timer = setInterval(refresh, REFRESH_MS);
    return () => {
      clearInterval(timer);
    };
  }, []);

  const visible = useMemo(() => filterLogs(entries, shown, query), [entries, shown, query]);

  return (
    <div className={styles["console-container"]}>
      <ConsoleToolbar
        searchQuery={query}
        onSearch={setQuery}
        shown={shown}
        onToggle={(level) => {
          setShown((current) => {
            const next = new Set(current);
            if (!next.delete(level)) {
              next.add(level);
            }
            return next;
          });
        }}
      />
      {/* The newest rows only: a long log would make the table slow to draw. */}
      <ConsoleTable filteredLogs={visible.slice(-MAX_ROWS)} />
      <ConsoleFooter
        filteredCount={visible.length}
        totalCount={entries.length}
        onClear={() => {
          clearLogs().then(() => {
            setEntries([]);
          }, log.error);
        }}
        onCopy={() => {
          navigator.clipboard.writeText(formatLogs(visible)).catch(log.error);
        }}
        onExport={() => {
          downloadText("NextTabletDriver_logs.txt", formatLogs(entries));
        }}
      />
    </div>
  );
}
