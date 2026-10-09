import { describe, it, expect } from "vitest";
import { render, screen, fireEvent } from "@testing-library/svelte";
import { fakeMapping } from "../../../test-support/fakeMapping";
import BindingCard from "../BindingCard.svelte";
import ButtonActions from "../ButtonActions.svelte";

describe("BindingCard", () => {
    it("edits the tip binding", async () => {
        const { mapping, markDirty, config } = fakeMapping({ tip_binding: "Left Click", eraser_binding: "None" });
        render(BindingCard, { props: { mapping, type: "tip" } });
        expect(screen.getByText("Tip Binding")).toBeInTheDocument();
        await fireEvent.input(screen.getByDisplayValue("Left Click"), { target: { value: "Right Click" } });
        expect(config?.tip_binding).toBe("Right Click");
        expect(config?.eraser_binding).toBe("None");
        expect(markDirty).toHaveBeenCalledOnce();
    });

    it("edits the eraser binding", async () => {
        const { mapping, markDirty, config } = fakeMapping({ tip_binding: "Left Click", eraser_binding: "Middle Click" });
        render(BindingCard, { props: { mapping, type: "eraser" } });
        expect(screen.getByText("Eraser Binding")).toBeInTheDocument();
        expect(screen.queryByDisplayValue("Left Click")).toBeNull();
        await fireEvent.input(screen.getByDisplayValue("Middle Click"), { target: { value: "None" } });
        expect(config?.eraser_binding).toBe("None");
        expect(config?.tip_binding).toBe("Left Click");
        expect(markDirty).toHaveBeenCalledOnce();
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

    it("writes the new binding and marks the profile dirty", async () => {
        const { mapping, markDirty, config } = fakeMapping({ pen_button_bindings: ["Right Click", "Middle Click"] });
        render(ButtonActions, { props: { mapping } });
        await fireEvent.input(screen.getByLabelText("Pen Button 1"), { target: { value: "Back" } });
        expect(config?.pen_button_bindings).toEqual(["Back", "Middle Click"]);
        expect(markDirty).toHaveBeenCalledOnce();
    });

    it("shows only the header while the config is not loaded", () => {
        const { mapping } = fakeMapping(null);
        render(ButtonActions, { props: { mapping } });
        expect(screen.getByText("Button Actions")).toBeInTheDocument();
        expect(screen.queryAllByRole("textbox")).toHaveLength(0);
    });
});
