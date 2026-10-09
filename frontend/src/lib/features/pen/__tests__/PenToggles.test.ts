import { describe, it, expect } from "vitest";
import { render, screen, fireEvent } from "@testing-library/svelte";
import { fakeMapping } from "../../../test-support/fakeMapping";
import HardwareToggles from "../HardwareToggles.svelte";
import Thresholds from "../Thresholds.svelte";

describe("HardwareToggles", () => {
    it("reflects the current pressure and tilt switches", () => {
        const { mapping } = fakeMapping({ disable_pressure: true, disable_tilt: false });
        render(HardwareToggles, { props: { mapping } });
        expect(screen.getByLabelText("Disable Pressure")).toBeChecked();
        expect(screen.getByLabelText("Disable Tilt")).not.toBeChecked();
    });

    it("writes the flipped switch and marks the profile dirty", async () => {
        const { mapping, markDirty, config } = fakeMapping({ disable_pressure: false, disable_tilt: false });
        render(HardwareToggles, { props: { mapping } });
        await fireEvent.click(screen.getByLabelText("Disable Tilt"));
        expect(config).toEqual({ disable_pressure: false, disable_tilt: true });
        expect(markDirty).toHaveBeenCalledOnce();
    });

    it("shows no switch until the config is loaded", () => {
        const { mapping } = fakeMapping(null);
        render(HardwareToggles, { props: { mapping } });
        expect(screen.queryByLabelText("Disable Pressure")).toBeNull();
        expect(screen.getByText("Hardware Toggles")).toBeInTheDocument();
    });
});

describe("Thresholds", () => {
    it("shows the tip and eraser thresholds", () => {
        const { mapping } = fakeMapping({ tip_threshold: 12, eraser_threshold: 30 });
        render(Thresholds, { props: { mapping } });
        expect(screen.getByDisplayValue("12")).toBeInTheDocument();
        expect(screen.getByDisplayValue("30")).toBeInTheDocument();
    });

    it("writes the edited threshold and marks the profile dirty", async () => {
        const { mapping, markDirty, config } = fakeMapping({ tip_threshold: 12, eraser_threshold: 30 });
        render(Thresholds, { props: { mapping } });
        await fireEvent.input(screen.getByDisplayValue("12"), { target: { value: "20" } });
        expect(config).toEqual({ tip_threshold: 20, eraser_threshold: 30 });
        expect(markDirty).toHaveBeenCalledOnce();
    });

    it("shows no field until the config is loaded", () => {
        const { mapping } = fakeMapping(null);
        render(Thresholds, { props: { mapping } });
        expect(screen.queryAllByRole("spinbutton")).toHaveLength(0);
        expect(screen.getByText("Thresholds")).toBeInTheDocument();
    });
});
