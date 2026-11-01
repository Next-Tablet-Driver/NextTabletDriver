import { describe, expect, it } from "vitest";
import {
  LEVELS,
  filterLogs,
  formatLine,
  formatLogs,
  levelOf,
  type Level,
} from "@/features/console/logs";
import type { LogEntry } from "@/infrastructure/tauri/commands";

const entry = (level: string, message: string, group = "HID"): LogEntry => ({
  time: "10:00:00",
  level,
  group,
  message,
  search_text: `${group} ${message}`.toLowerCase(),
});

const logs = [
  entry("INFO", "Tablet connected"),
  entry("WARN", "Slow enumeration"),
  entry("ERROR", "Open failed", "Driver"),
  entry("DEBUG", "Packet parsed"),
  entry("TRACE", "Something odd"),
];

const all = new Set<Level>(LEVELS);

describe("levelOf", () => {
  it("reads the level in any case and shows unknown ones as Debug", () => {
    expect(levelOf(entry("warn", ""))).toBe("Warn");
    expect(levelOf(entry("TRACE", ""))).toBe("Debug");
  });
});

describe("filterLogs", () => {
  it("keeps everything when every level is shown and the search is empty", () => {
    expect(filterLogs(logs, all, "  ")).toHaveLength(5);
  });

  it("hides the levels that are switched off", () => {
    const shown = new Set<Level>(["Error"]);
    expect(filterLogs(logs, shown, "").map((l) => l.message)).toEqual(["Open failed"]);
  });

  it("searches the group and the message, ignoring the case", () => {
    expect(filterLogs(logs, all, "DRIVER").map((l) => l.message)).toEqual(["Open failed"]);
    expect(filterLogs(logs, all, "tablet")).toHaveLength(1);
  });

  it("shows nothing when no level is shown", () => {
    expect(filterLogs(logs, new Set(), "")).toEqual([]);
  });
});

describe("formatLogs", () => {
  it("writes one bracketed line per entry", () => {
    expect(formatLine(entry("INFO", "Hello"))).toBe("[10:00:00] [INFO] [HID] Hello");
    expect(formatLogs(logs.slice(0, 2)).split("\n")).toHaveLength(2);
  });
});
