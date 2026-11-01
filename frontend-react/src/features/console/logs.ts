import type { LogEntry } from "@/infrastructure/tauri/commands";

export const LEVELS = ["Info", "Warn", "Error", "Debug"] as const;
export type Level = (typeof LEVELS)[number];

/** The most rows drawn at once: the newest ones, so the page stays fast with a long log. */
export const MAX_ROWS = 1500;

/** A level the backend names in any case; anything unknown is shown as `Debug`. */
export function levelOf(entry: LogEntry): Level {
  const upper = entry.level.toUpperCase();
  return LEVELS.find((level) => level.toUpperCase() === upper) ?? "Debug";
}

/** The entries to show: the chosen levels, and, when `query` is set, those that contain it. */
export function filterLogs(
  entries: readonly LogEntry[],
  shown: ReadonlySet<Level>,
  query: string,
): LogEntry[] {
  const needle = query.trim().toLowerCase();
  return entries.filter(
    (entry) => shown.has(levelOf(entry)) && (needle === "" || entry.search_text.includes(needle)),
  );
}

/** One log line as text, for the clipboard and the exported file. */
export function formatLine(entry: LogEntry): string {
  return `[${entry.time}] [${entry.level}] [${entry.group}] ${entry.message}`;
}

export function formatLogs(entries: readonly LogEntry[]): string {
  return entries.map(formatLine).join("\n");
}

/** Saves text as a file through the browser. */
export function downloadText(fileName: string, text: string): void {
  const url = URL.createObjectURL(new Blob([text], { type: "text/plain" }));
  const link = document.createElement("a");
  link.href = url;
  link.download = fileName;
  link.click();
  URL.revokeObjectURL(url);
}
