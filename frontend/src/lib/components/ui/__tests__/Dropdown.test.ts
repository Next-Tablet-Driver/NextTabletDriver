import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/svelte";
import { createRawSnippet } from "svelte";
import DropdownItem from "../DropdownItem.svelte";
import DropdownMenu from "../DropdownMenu.svelte";

const label = createRawSnippet(() => ({ render: () => "<b>Open profile</b>" }));
const shortcut = createRawSnippet(() => ({ render: () => "<kbd>Ctrl+O</kbd>" }));

describe("DropdownItem", () => {
    it("renders its content as a menu item", () => {
        render(DropdownItem, { props: { children: label } });
        expect(screen.getByRole("menuitem")).toHaveTextContent("Open profile");
    });

    it("calls onclick when clicked", async () => {
        const onclick = vi.fn();
        render(DropdownItem, { props: { children: label, onclick } });
        await fireEvent.click(screen.getByRole("menuitem"));
        expect(onclick).toHaveBeenCalledOnce();
    });

    it("activates with Enter and Space but ignores other keys", async () => {
        const onclick = vi.fn();
        render(DropdownItem, { props: { children: label, onclick } });
        const item = screen.getByRole("menuitem");
        await fireEvent.keyDown(item, { key: "Enter" });
        await fireEvent.keyDown(item, { key: " " });
        expect(onclick).toHaveBeenCalledTimes(2);
        await fireEvent.keyDown(item, { key: "a" });
        expect(onclick).toHaveBeenCalledTimes(2);
    });

    it("does nothing when disabled, and leaves the tab order", async () => {
        const onclick = vi.fn();
        render(DropdownItem, { props: { children: label, onclick, disabled: true } });
        const item = screen.getByRole("menuitem");
        expect(item).toHaveClass("disabled");
        expect(item).toHaveAttribute("tabindex", "-1");
        await fireEvent.click(item);
        await fireEvent.keyDown(item, { key: "Enter" });
        expect(onclick).not.toHaveBeenCalled();
    });

    it("is focusable when enabled", () => {
        render(DropdownItem, { props: { children: label } });
        expect(screen.getByRole("menuitem")).toHaveAttribute("tabindex", "0");
    });

    it("shows a chevron for submenus, or the right-hand content when given", () => {
        const { container, unmount } = render(DropdownItem, { props: { children: label, hasSubmenu: true } });
        expect(container.querySelector(".arrow svg")).not.toBeNull();
        expect(container.querySelector(".has-submenu")).not.toBeNull();
        unmount();

        const { container: other } = render(DropdownItem, {
            props: { children: label, hasSubmenu: true, rightContent: shortcut },
        });
        expect(other.querySelector(".right")).toHaveTextContent("Ctrl+O");
        expect(other.querySelector(".arrow")).toBeNull();
    });

    it("forwards pointer enter and leave", async () => {
        const onmouseenter = vi.fn();
        const onmouseleave = vi.fn();
        render(DropdownItem, { props: { children: label, onmouseenter, onmouseleave } });
        const item = screen.getByRole("menuitem");
        await fireEvent.mouseEnter(item);
        await fireEvent.mouseLeave(item);
        expect(onmouseenter).toHaveBeenCalledOnce();
        expect(onmouseleave).toHaveBeenCalledOnce();
    });
});

describe("DropdownMenu", () => {
    beforeEach(() => {
        // jsdom has no layout engine: only the observer API is needed for the menu to mount.
        vi.stubGlobal(
            "ResizeObserver",
            class {
                observe = vi.fn();
                disconnect = vi.fn();
            },
        );
    });
    afterEach(() => {
        vi.unstubAllGlobals();
    });

    it("renders its children inside a menu", () => {
        render(DropdownMenu, { props: { children: label } });
        expect(screen.getByRole("menu")).toHaveTextContent("Open profile");
    });

    it("opens below its anchor by default", () => {
        render(DropdownMenu, { props: { children: label } });
        expect(screen.getByRole("menu")).toHaveAttribute("data-side", "bottom");
        expect(screen.getByRole("menu")).toHaveAttribute("data-align", "start");
    });

    it("honours the requested side and extra class", () => {
        render(DropdownMenu, { props: { children: label, placement: "right-start", class: "wide" } });
        const menu = screen.getByRole("menu");
        expect(menu).toHaveAttribute("data-side", "right");
        expect(menu).toHaveClass("dropdown-menu", "wide");
    });

    it("stops observing when it is removed", () => {
        const disconnect = vi.fn();
        vi.stubGlobal(
            "ResizeObserver",
            class {
                observe = vi.fn();
                disconnect = disconnect;
            },
        );
        const { unmount } = render(DropdownMenu, { props: { children: label } });
        unmount();
        expect(disconnect).toHaveBeenCalled();
    });
});
