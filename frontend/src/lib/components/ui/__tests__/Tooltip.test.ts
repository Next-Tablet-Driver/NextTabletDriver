import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/svelte";
import { createRawSnippet } from "svelte";
import Tooltip from "../Tooltip.svelte";

const children = createRawSnippet(() => ({ render: () => "<span>hover me</span>" }));

function wrapperOf(): HTMLElement {
    const element = screen.getByText("hover me").parentElement;
    if (element === null) throw new Error("the tooltip wrapper is missing");
    return element;
}

describe("Tooltip", () => {
    beforeEach(() => {
        vi.useFakeTimers();
    });
    afterEach(() => {
        vi.useRealTimers();
    });

    it("renders its children without showing the tip", () => {
        render(Tooltip, { props: { text: "Helpful", children } });
        expect(screen.getByText("hover me")).toBeInTheDocument();
        expect(screen.queryByText("Helpful")).toBeNull();
    });

    it("shows the tip after the delay, next to the pointer", async () => {
        render(Tooltip, { props: { text: "Helpful", delay: 500, children } });
        await fireEvent.mouseEnter(wrapperOf(), {
            clientX: 100,
            clientY: 40,
        });
        await vi.advanceTimersByTimeAsync(499);
        expect(screen.queryByText("Helpful")).toBeNull();
        await vi.advanceTimersByTimeAsync(2);
        const tip = screen.getByText("Helpful");
        expect(tip).toHaveStyle({ top: "60px", left: "110px" });
    });

    it("uses a one second delay by default", async () => {
        render(Tooltip, { props: { text: "Helpful", children } });
        await fireEvent.mouseEnter(wrapperOf());
        await vi.advanceTimersByTimeAsync(999);
        expect(screen.queryByText("Helpful")).toBeNull();
        await vi.advanceTimersByTimeAsync(2);
        expect(screen.getByText("Helpful")).toBeInTheDocument();
    });

    it("cancels a pending tip when the pointer leaves", async () => {
        render(Tooltip, { props: { text: "Helpful", delay: 500, children } });
        const wrapper = wrapperOf();
        await fireEvent.mouseEnter(wrapper);
        await fireEvent.mouseLeave(wrapper);
        await vi.advanceTimersByTimeAsync(1000);
        expect(screen.queryByText("Helpful")).toBeNull();
    });

    it("hides a visible tip when the pointer leaves", async () => {
        render(Tooltip, { props: { text: "Helpful", delay: 10, children } });
        const wrapper = wrapperOf();
        await fireEvent.mouseEnter(wrapper);
        await vi.advanceTimersByTimeAsync(20);
        expect(screen.getByText("Helpful")).toBeInTheDocument();
        await fireEvent.mouseLeave(wrapper);
        expect(screen.queryByText("Helpful")).toBeNull();
    });

    it("never shows anything when disabled or without text", async () => {
        const { unmount } = render(Tooltip, { props: { text: "Helpful", delay: 10, disabled: true, children } });
        await fireEvent.mouseEnter(wrapperOf());
        await vi.advanceTimersByTimeAsync(50);
        expect(screen.queryByText("Helpful")).toBeNull();
        unmount();

        render(Tooltip, { props: { text: "", delay: 10, children } });
        const wrapper = wrapperOf();
        await fireEvent.mouseEnter(wrapper);
        await vi.advanceTimersByTimeAsync(50);
        expect(wrapper.querySelector(".tooltip-content")).toBeNull();
    });

    it("follows the pointer until the tip appears", async () => {
        render(Tooltip, { props: { text: "Helpful", delay: 500, children } });
        const wrapper = wrapperOf();
        await fireEvent.mouseEnter(wrapper, { clientX: 10, clientY: 10 });
        await fireEvent.mouseMove(wrapper, { clientX: 200, clientY: 100 });
        await vi.advanceTimersByTimeAsync(600);
        expect(screen.getByText("Helpful")).toHaveStyle({ top: "120px", left: "210px" });
    });
});
