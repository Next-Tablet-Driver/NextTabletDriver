import { describe, it, expect, vi, type Mock } from "vitest";
import { render, screen, fireEvent } from "@testing-library/svelte";
import type { MappingState } from "../../mapping/mapping-state.svelte";
import HardwareToggles from "../HardwareToggles.svelte";
import Thresholds from "../Thresholds.svelte";

function fakeMapping(config: Record<string, unknown> | null): { mapping: MappingState; markDirty: Mock } {
    const markDirty = vi.fn();
    return { mapping: { config, markDirty } as unknown as MappingState, markDirty };
}

describe("HardwareToggles", () => {
    it("reflects the current pressure and tilt switches", () => {
        const { mapping } = fakeMapping({ disable_pressure: true, disable_tilt: false });
        render(HardwareToggles, { props: { mapping } });
        expect(screen.getByLabelText("Disable Pressure")).toBeChecked();
        expect(screen.getByLabelText("Disable Tilt")).not.toBeChecked();
    });

    it("marks the profile dirty when a switch is flipped", async () => {
        const { mapping, markDirty } = fakeMapping({ disable_pressure: false, disable_tilt: false });
        render(HardwareToggles, { props: { mapping } });
        await fireEvent.click(screen.getByLabelText("Disable Tilt"));
        expect(markDirty).toHaveBeenCalled();
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

    it("marks the profile dirty when a threshold is edited", async () => {
        const { mapping, markDirty } = fakeMapping({ tip_threshold: 12, eraser_threshold: 30 });
        render(Thresholds, { props: { mapping } });
        await fireEvent.input(screen.getByDisplayValue("12"), { target: { value: "20" } });
        expect(markDirty).toHaveBeenCalled();
    });

    it("shows no field until the config is loaded", () => {
        const { mapping } = fakeMapping(null);
        render(Thresholds, { props: { mapping } });
        expect(screen.queryAllByRole("spinbutton")).toHaveLength(0);
        expect(screen.getByText("Thresholds")).toBeInTheDocument();
    });
});
