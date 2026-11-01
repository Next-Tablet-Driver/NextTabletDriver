import { useEffect, useMemo, useState } from "react";
import { Copy01, Download01, SearchLg, Trash01 } from "@untitledui/icons";
import { Badge } from "@/components/base/badges/badges";
import { ButtonGroup, ButtonGroupItem } from "@/components/base/button-group/button-group";
import { Button } from "@/components/base/buttons/button";
import { Input } from "@/components/base/input/input";
import { Table, TableCard } from "@/components/application/table/table";
import {
  LEVELS,
  MAX_ROWS,
  downloadText,
  filterLogs,
  formatLogs,
  levelOf,
  type Level,
} from "@/features/console/logs";
import { log } from "@/infrastructure/log";
import { clearLogs, getLogs, type LogEntry } from "@/infrastructure/tauri/commands";
import { Page } from "@/ui/Page";

const REFRESH_MS = 500;

const LEVEL_COLOR = { Info: "blue", Warn: "warning", Error: "error", Debug: "gray" } as const;

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
  // The newest rows only: a long log would make the table slow to draw.
  const rows = visible.slice(-MAX_ROWS);
  const copy = (text: string): void => {
    navigator.clipboard.writeText(text).catch(log.error);
  };

  return (
    <Page title="Console" description="What the driver is doing, and what went wrong.">
      <div className="flex flex-wrap items-end justify-between gap-3">
        <div className="w-full max-w-sm">
          <Input
            aria-label="Search the logs"
            placeholder="Search logs…"
            icon={SearchLg}
            value={query}
            onChange={setQuery}
          />
        </div>
        <ButtonGroup
          aria-label="Levels to show"
          selectionMode="multiple"
          selectedKeys={shown as Set<string>}
          onSelectionChange={(keys) => {
            setShown(new Set(LEVELS.filter((level) => keys.has(level))));
          }}
        >
          {LEVELS.map((level) => (
            <ButtonGroupItem key={level} id={level}>
              {level}
            </ButtonGroupItem>
          ))}
        </ButtonGroup>
      </div>

      <TableCard.Root size="sm">
        <TableCard.Header
          title="Logs"
          badge={`${String(visible.length)} / ${String(entries.length)}`}
          contentTrailing={
            <div className="flex flex-wrap gap-2">
              <Button
                color="secondary"
                size="sm"
                iconLeading={Trash01}
                onPress={() => {
                  clearLogs().then(() => {
                    setEntries([]);
                  }, log.error);
                }}
              >
                Clear
              </Button>
              <Button
                color="secondary"
                size="sm"
                iconLeading={Copy01}
                onPress={() => {
                  copy(formatLogs(visible));
                }}
              >
                Copy visible
              </Button>
              <Button
                color="secondary"
                size="sm"
                iconLeading={Download01}
                onPress={() => {
                  downloadText("NextTabletDriver_logs.txt", formatLogs(entries));
                }}
              >
                Export all
              </Button>
            </div>
          }
        />
        <div className="max-h-[28rem] overflow-auto">
          <Table aria-label="Logs" size="sm">
            <Table.Header>
              <Table.Head id="time" isRowHeader label="Time" />
              <Table.Head id="level" label="Level" />
              <Table.Head id="group" label="Group" />
              <Table.Head id="message" label="Message" />
            </Table.Header>
            <Table.Body
              items={rows.map((entry, index) => ({ id: index, entry }))}
              renderEmptyState={() => (
                <p className="px-6 py-8 text-center text-sm text-tertiary">No logs to display</p>
              )}
            >
              {({ id, entry }) => (
                <Table.Row id={id}>
                  <Table.Cell className="font-mono whitespace-nowrap">{entry.time}</Table.Cell>
                  <Table.Cell>
                    <Badge type="pill-color" size="sm" color={LEVEL_COLOR[levelOf(entry)]}>
                      {entry.level}
                    </Badge>
                  </Table.Cell>
                  <Table.Cell className="whitespace-nowrap">{entry.group}</Table.Cell>
                  <Table.Cell className="font-mono break-words whitespace-pre-wrap text-secondary">
                    {entry.message}
                  </Table.Cell>
                </Table.Row>
              )}
            </Table.Body>
          </Table>
        </div>
      </TableCard.Root>
    </Page>
  );
}
