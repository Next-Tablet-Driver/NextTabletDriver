import { describe, it, expect } from "vitest";
import { render, screen, fireEvent } from "@testing-library/svelte";
import Checkbox from "../Checkbox.svelte";

describe("Checkbox", () => {
    it("renders its label and reflects the checked prop", () => {
        render(Checkbox, { props: { label: "Lock aspect ratio", checked: true } });
        expect(screen.getByLabelText("Lock aspect ratio")).toBeChecked();
    });

    it("toggles when clicked", async () => {
        render(Checkbox, { props: { label: "Snap to edges", checked: false } });
        const box = screen.getByLabelText("Snap to edges");
        expect(box).not.toBeChecked();
        await fireEvent.click(box);
        expect(box).toBeChecked();
    });

    it("forwards extra attributes to the input", () => {
        render(Checkbox, { props: { label: "Disabled", disabled: true } });
        expect(screen.getByLabelText("Disabled")).toBeDisabled();
    });
});
