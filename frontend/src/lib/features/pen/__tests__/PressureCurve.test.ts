import { describe, it, expect, vi, type Mock } from "vitest";
import { render, screen, fireEvent } from "@testing-library/svelte";
import type { MappingState } from "../../mapping/mapping-state.svelte";
import PressureCurve from "../PressureCurve.svelte";

interface CurveConfig {
    pressure_curve: { curve_type: string };
}

function fakeMapping(curveType: string | null): { mapping: MappingState; markDirty: Mock; config: CurveConfig | null } {
    const markDirty = vi.fn();
    const config = curveType === null ? null : { pressure_curve: { curve_type: curveType } };
    return { mapping: { config, markDirty } as unknown as MappingState, markDirty, config };
}

describe("PressureCurve", () => {
    it("offers the three curve types and highlights the current one", () => {
        const { mapping } = fakeMapping("Exponential");
        render(PressureCurve, { props: { mapping } });
        expect(screen.getByText("Pressure Curve")).toBeInTheDocument();
        expect(screen.getByRole("button", { name: "Exponential" })).toHaveClass("primary");
        expect(screen.getByRole("button", { name: "Linear" })).toHaveClass("ghost");
        expect(screen.getByRole("button", { name: "Custom" })).toHaveClass("ghost");
    });

    it("selects the clicked curve and marks the profile dirty", async () => {
        const { mapping, markDirty, config } = fakeMapping("Linear");
        render(PressureCurve, { props: { mapping } });
        await fireEvent.click(screen.getByRole("button", { name: "Custom" }));
        expect(config?.pressure_curve.curve_type).toBe("Custom");
        expect(markDirty).toHaveBeenCalledOnce();
    });

    it("can switch back to every curve", async () => {
        const { mapping, config } = fakeMapping("Custom");
        render(PressureCurve, { props: { mapping } });
        await fireEvent.click(screen.getByRole("button", { name: "Exponential" }));
        expect(config?.pressure_curve.curve_type).toBe("Exponential");
        await fireEvent.click(screen.getByRole("button", { name: "Linear" }));
        expect(config?.pressure_curve.curve_type).toBe("Linear");
    });

    it("shows the raw and curve bars, with no selector before the config is loaded", () => {
        const { mapping } = fakeMapping(null);
        render(PressureCurve, { props: { mapping } });
        expect(screen.getByText("Raw")).toBeInTheDocument();
        expect(screen.getByText("Curve")).toBeInTheDocument();
        expect(screen.queryAllByRole("button")).toHaveLength(0);
    });
});
