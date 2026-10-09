import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/svelte";
import type { LogEntry } from "../../../infrastructure/tauri/commands";
import ConsoleTable from "../ConsoleTable.svelte";

function log(level: string, message: string, time = "12:00:00"): LogEntry {
    return { time, level, group: "Driver", message } as LogEntry;
}

describe("ConsoleTable", () => {
    it("shows the column headers", () => {
        render(ConsoleTable, { props: { filteredLogs: [] } });
        for (const header of ["Time", "Level", "Group", "Message"]) {
            expect(screen.getByRole("columnheader", { name: header })).toBeInTheDocument();
        }
    });

    it("renders one row per log with all its fields", () => {
        render(ConsoleTable, {
            props: { filteredLogs: [log("INFO", "Tablet connected", "10:00:01"), log("ERROR", "Read failed", "10:00:02")] },
        });
        expect(screen.getByText("Tablet connected")).toBeInTheDocument();
        expect(screen.getByText("Read failed")).toBeInTheDocument();
        expect(screen.getByText("10:00:01")).toBeInTheDocument();
        expect(screen.getAllByText("Driver")).toHaveLength(2);
        expect(screen.queryByText("No logs to display")).toBeNull();
    });

    it("says so when there is nothing to display", () => {
        render(ConsoleTable, { props: { filteredLogs: [] } });
        expect(screen.getByText("No logs to display")).toBeInTheDocument();
    });

    it("colours the level, case-insensitively, defaulting to debug", () => {
        render(ConsoleTable, {
            props: {
                filteredLogs: [log("info", "a"), log("WARN", "b"), log("Error", "c"), log("TRACE", "d")],
            },
        });
        expect(screen.getByText("info")).toHaveStyle({ color: "var(--info)" });
        expect(screen.getByText("WARN")).toHaveStyle({ color: "var(--warning)" });
        expect(screen.getByText("Error")).toHaveStyle({ color: "var(--error)" });
        expect(screen.getByText("TRACE")).toHaveStyle({ color: "var(--debug)" });
    });
});
