import { describe, it, expect, vi, type Mock } from "vitest";
import { render, screen, fireEvent } from "@testing-library/svelte";
import ConsoleFooter from "../ConsoleFooter.svelte";

function setup(counts = { filteredCount: 12, totalCount: 340 }): Record<"clearConsole" | "copyVisible" | "exportAll", Mock> {
    const handlers = { clearConsole: vi.fn(), copyVisible: vi.fn(), exportAll: vi.fn() };
    render(ConsoleFooter, { props: { ...handlers, ...counts } });
    return handlers;
}

describe("ConsoleFooter", () => {
    it("reports how many logs are shown out of the total", () => {
        setup();
        expect(screen.getByText(/Showing 12 \/ 340 logs/)).toBeInTheDocument();
    });

    it("runs the matching action for each button", async () => {
        const h = setup();
        await fireEvent.click(screen.getByRole("button", { name: /Clear Console/ }));
        expect(h.clearConsole).toHaveBeenCalledOnce();
        expect(h.copyVisible).not.toHaveBeenCalled();

        await fireEvent.click(screen.getByRole("button", { name: /Copy Visible/ }));
        expect(h.copyVisible).toHaveBeenCalledOnce();

        await fireEvent.click(screen.getByRole("button", { name: /Export All/ }));
        expect(h.exportAll).toHaveBeenCalledOnce();
    });

    it("handles an empty console", () => {
        setup({ filteredCount: 0, totalCount: 0 });
        expect(screen.getByText(/Showing 0 \/ 0 logs/)).toBeInTheDocument();
    });
});
