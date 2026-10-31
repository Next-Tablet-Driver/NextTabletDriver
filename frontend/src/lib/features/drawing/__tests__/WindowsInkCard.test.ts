import { describe, it, expect } from "vitest";
import { render, screen, fireEvent } from "@testing-library/svelte";
import { fakeMapping } from "../../../test-support/fakeMapping";
import WindowsInkCard from "../WindowsInkCard.svelte";

describe("WindowsInkCard", () => {
    it("offers the mouse and Windows Ink", () => {
        const { mapping } = fakeMapping({ windows_ink: false, mode: "Absolute" });
        render(WindowsInkCard, { props: { mapping } });
        expect(screen.getAllByRole("radio")).toHaveLength(2);
        expect(screen.getByRole("radio", { name: "Mouse" })).toBeInTheDocument();
        expect(screen.getByRole("radio", { name: "Windows Ink" })).toBeInTheDocument();
    });

    it("selects the mouse for a profile that never asked for Windows Ink", () => {
        const { mapping } = fakeMapping({ windows_ink: false, mode: "Absolute" });
        render(WindowsInkCard, { props: { mapping } });
        expect(screen.getByRole("radio", { name: "Mouse" })).toBeChecked();
        expect(screen.getByRole("radio", { name: "Windows Ink" })).not.toBeChecked();
    });

    it("selects Windows Ink when the profile asks for it", () => {
        const { mapping } = fakeMapping({ windows_ink: true, mode: "Absolute" });
        render(WindowsInkCard, { props: { mapping } });
        expect(screen.getByRole("radio", { name: "Windows Ink" })).toBeChecked();
    });

    it("writes the chosen mode and marks the profile dirty", async () => {
        const { mapping, markDirty, config } = fakeMapping({ windows_ink: false, mode: "Absolute" });
        render(WindowsInkCard, { props: { mapping } });
        await fireEvent.click(screen.getByRole("radio", { name: "Windows Ink" }));
        expect(config).toEqual({ windows_ink: true, mode: "Absolute" });
        expect(markDirty).toHaveBeenCalledOnce();
    });

    it("does not mark the profile dirty when the chosen mode is already the current one", async () => {
        const { mapping, markDirty } = fakeMapping({ windows_ink: false, mode: "Absolute" });
        render(WindowsInkCard, { props: { mapping } });
        await fireEvent.click(screen.getByRole("radio", { name: "Mouse" }));
        expect(markDirty).not.toHaveBeenCalled();
    });

    it("warns that relative mode keeps the mouse", () => {
        const { mapping } = fakeMapping({ windows_ink: true, mode: "Relative" });
        render(WindowsInkCard, { props: { mapping } });
        expect(screen.getByRole("status")).toHaveTextContent("Relative mode");
    });

    it("does not warn in absolute mode", () => {
        const { mapping } = fakeMapping({ windows_ink: true, mode: "Absolute" });
        render(WindowsInkCard, { props: { mapping } });
        expect(screen.queryByRole("status")).toBeNull();
    });

    it("does not warn while the mouse is selected", () => {
        const { mapping } = fakeMapping({ windows_ink: false, mode: "Relative" });
        render(WindowsInkCard, { props: { mapping } });
        expect(screen.queryByRole("status")).toBeNull();
    });

    it("shows nothing until the config is loaded", () => {
        const { mapping } = fakeMapping(null);
        render(WindowsInkCard, { props: { mapping } });
        expect(screen.queryAllByRole("radio")).toHaveLength(0);
    });
});
