import { describe, it, expect, vi, type Mock } from "vitest";
import { render, screen, fireEvent } from "@testing-library/svelte";
import type { MappingState } from "../../mapping/mapping-state.svelte";
import BindingCard from "../BindingCard.svelte";
import ButtonActions from "../ButtonActions.svelte";

function fakeMapping(config: Record<string, unknown> | null): { mapping: MappingState; markDirty: Mock } {
    const markDirty = vi.fn();
    return { mapping: { config, markDirty } as unknown as MappingState, markDirty };
}

describe("BindingCard", () => {
    it("edits the tip binding", async () => {
        const { mapping, markDirty } = fakeMapping({ tip_binding: "Left Click", eraser_binding: "None" });
        render(BindingCard, { props: { mapping, type: "tip" } });
        expect(screen.getByText("Tip Binding")).toBeInTheDocument();
        const input = screen.getByDisplayValue("Left Click");
        await fireEvent.input(input, { target: { value: "Right Click" } });
        expect(markDirty).toHaveBeenCalled();
        expect(input).toHaveValue("Right Click");
    });

    it("edits the eraser binding", () => {
        const { mapping } = fakeMapping({ tip_binding: "Left Click", eraser_binding: "Middle Click" });
        render(BindingCard, { props: { mapping, type: "eraser" } });
        expect(screen.getByText("Eraser Binding")).toBeInTheDocument();
        expect(screen.getByDisplayValue("Middle Click")).toBeInTheDocument();
        expect(screen.queryByDisplayValue("Left Click")).toBeNull();
    });

    it("offers an edit button labelled with the binding kind", () => {
        const { mapping } = fakeMapping({ tip_binding: "", eraser_binding: "" });
        render(BindingCard, { props: { mapping, type: "eraser" } });
        expect(screen.getByRole("button", { name: "Edit eraser Binding" })).toBeInTheDocument();
    });

    it("shows no field until the config is loaded", () => {
        const { mapping } = fakeMapping(null);
        render(BindingCard, { props: { mapping, type: "tip" } });
        expect(screen.queryAllByRole("textbox")).toHaveLength(0);
    });
});

describe("ButtonActions", () => {
    it("lists one field per pen button", () => {
        const { mapping } = fakeMapping({ pen_button_bindings: ["Right Click", "Middle Click", "None"] });
        render(ButtonActions, { props: { mapping } });
        expect(screen.getByLabelText("Pen Button 1")).toHaveValue("Right Click");
        expect(screen.getByLabelText("Pen Button 2")).toHaveValue("Middle Click");
        expect(screen.getByLabelText("Pen Button 3")).toHaveValue("None");
    });

    it("marks the profile dirty when a binding changes", async () => {
        const { mapping, markDirty } = fakeMapping({ pen_button_bindings: ["Right Click"] });
        render(ButtonActions, { props: { mapping } });
        await fireEvent.input(screen.getByLabelText("Pen Button 1"), { target: { value: "Back" } });
        expect(markDirty).toHaveBeenCalled();
    });

    it("shows only the header while the config is not loaded", () => {
        const { mapping } = fakeMapping(null);
        render(ButtonActions, { props: { mapping } });
        expect(screen.getByText("Button Actions")).toBeInTheDocument();
        expect(screen.queryAllByRole("textbox")).toHaveLength(0);
    });
});
