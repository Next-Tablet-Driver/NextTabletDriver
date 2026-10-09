import { describe, it, expect } from "vitest";
import { render, screen, fireEvent } from "@testing-library/svelte";
import Tabs from "../Tabs.svelte";

describe("Tabs", () => {
    it("renders one button per tab and marks the active one", () => {
        render(Tabs, { props: { tabs: ["Mapping", "Pen", "Filters"], activeTab: "Pen" } });
        expect(screen.getAllByRole("button").map((b) => b.textContent.trim())).toEqual(["Mapping", "Pen", "Filters"]);
        expect(screen.getByRole("button", { name: "Pen" })).toHaveClass("active");
        expect(screen.getByRole("button", { name: "Mapping" })).not.toHaveClass("active");
    });

    it("switches the active tab when a tab is clicked", async () => {
        render(Tabs, { props: { tabs: ["Mapping", "Pen"], activeTab: "Mapping" } });
        await fireEvent.click(screen.getByRole("button", { name: "Pen" }));
        expect(screen.getByRole("button", { name: "Pen" })).toHaveClass("active");
        expect(screen.getByRole("button", { name: "Mapping" })).not.toHaveClass("active");
    });

    it("renders no button without tabs", () => {
        render(Tabs, { props: { tabs: [], activeTab: "" } });
        expect(screen.queryAllByRole("button")).toHaveLength(0);
    });
});
