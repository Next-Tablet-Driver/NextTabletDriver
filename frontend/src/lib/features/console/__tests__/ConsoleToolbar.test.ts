import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/svelte";
import ConsoleToolbar from "../ConsoleToolbar.svelte";

const ALL_ON = { Info: true, Warn: true, Error: true, Debug: false };

describe("ConsoleToolbar", () => {
    it("shows the current search and highlights the enabled levels", () => {
        render(ConsoleToolbar, {
            props: { searchQuery: "tablet", filters: ALL_ON, toggleFilter: vi.fn() },
        });
        expect(screen.getByPlaceholderText("Search logs...")).toHaveValue("tablet");
        expect(screen.getByRole("button", { name: "Info" })).toHaveClass("active");
        expect(screen.getByRole("button", { name: "Warn" })).toHaveClass("active");
        expect(screen.getByRole("button", { name: "Error" })).toHaveClass("active");
        expect(screen.getByRole("button", { name: "Debug" })).not.toHaveClass("active");
    });

    it("toggles the level that was clicked", async () => {
        const toggleFilter = vi.fn();
        render(ConsoleToolbar, { props: { filters: ALL_ON, toggleFilter } });
        await fireEvent.click(screen.getByRole("button", { name: "Debug" }));
        expect(toggleFilter).toHaveBeenLastCalledWith("Debug");
        await fireEvent.click(screen.getByRole("button", { name: "Info" }));
        expect(toggleFilter).toHaveBeenLastCalledWith("Info");
        await fireEvent.click(screen.getByRole("button", { name: "Warn" }));
        expect(toggleFilter).toHaveBeenLastCalledWith("Warn");
        await fireEvent.click(screen.getByRole("button", { name: "Error" }));
        expect(toggleFilter).toHaveBeenLastCalledWith("Error");
        expect(toggleFilter).toHaveBeenCalledTimes(4);
    });

    it("offers to clear the search only when there is one", async () => {
        render(ConsoleToolbar, { props: { filters: ALL_ON, toggleFilter: vi.fn() } });
        const input = screen.getByPlaceholderText("Search logs...");
        expect(screen.queryByRole("button", { name: "Clear search" })).toBeNull();

        await fireEvent.input(input, { target: { value: "usb" } });
        const clear = screen.getByRole("button", { name: "Clear search" });
        await fireEvent.click(clear);

        expect(input).toHaveValue("");
        expect(screen.queryByRole("button", { name: "Clear search" })).toBeNull();
    });
});
