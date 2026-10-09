import { describe, it, expect } from "vitest";
import { render, screen, fireEvent } from "@testing-library/svelte";
import TabBar from "../TabBar.svelte";

const LABELS = ["Output", "Filters", "Pen Settings", "Console", "Settings", "Release", "Credits"];

describe("TabBar", () => {
    it("lists every page of the app, in order", () => {
        render(TabBar);
        expect(screen.getAllByRole("button").map((b) => b.textContent.trim())).toEqual(LABELS);
    });

    it("starts on the Output page by default", () => {
        render(TabBar);
        expect(screen.getByRole("button", { name: "Output" })).toHaveClass("active");
    });

    it("honours the initial tab", () => {
        render(TabBar, { props: { activeTab: "console" } });
        expect(screen.getByRole("button", { name: "Console" })).toHaveClass("active");
        expect(screen.getByRole("button", { name: "Output" })).not.toHaveClass("active");
    });

    it("moves the highlight to the clicked tab", async () => {
        render(TabBar);
        await fireEvent.click(screen.getByRole("button", { name: "Pen Settings" }));
        expect(screen.getByRole("button", { name: "Pen Settings" })).toHaveClass("active");
        expect(screen.getByRole("button", { name: "Output" })).not.toHaveClass("active");
    });
});
