import { describe, it, expect } from "vitest";
import { render, screen, fireEvent } from "@testing-library/svelte";
import { fakeMapping, type FakeMapping } from "../../../test-support/fakeMapping";
import PressureCurve from "../PressureCurve.svelte";

interface CurveConfig {
    pressure_curve: { curve_type: string };
}

function curveMapping(curveType: string | null): FakeMapping<CurveConfig> {
    return fakeMapping(curveType === null ? null : { pressure_curve: { curve_type: curveType } });
}

describe("PressureCurve", () => {
    it("offers the three curve types and highlights the current one", () => {
        const { mapping } = curveMapping("Exponential");
        render(PressureCurve, { props: { mapping } });
        expect(screen.getByText("Pressure Curve")).toBeInTheDocument();
        expect(screen.getByRole("button", { name: "Exponential" })).toHaveClass("primary");
        expect(screen.getByRole("button", { name: "Linear" })).toHaveClass("ghost");
        expect(screen.getByRole("button", { name: "Custom" })).toHaveClass("ghost");
    });

    it("selects the clicked curve and marks the profile dirty", async () => {
        const { mapping, markDirty, config } = curveMapping("Linear");
        render(PressureCurve, { props: { mapping } });
        await fireEvent.click(screen.getByRole("button", { name: "Custom" }));
        expect(config?.pressure_curve.curve_type).toBe("Custom");
        expect(markDirty).toHaveBeenCalledOnce();
    });

    it("can switch back to every curve", async () => {
        const { mapping, config } = curveMapping("Custom");
        render(PressureCurve, { props: { mapping } });
        await fireEvent.click(screen.getByRole("button", { name: "Exponential" }));
        expect(config?.pressure_curve.curve_type).toBe("Exponential");
        await fireEvent.click(screen.getByRole("button", { name: "Linear" }));
        expect(config?.pressure_curve.curve_type).toBe("Linear");
    });

    it("shows the raw and curve bars, with no selector before the config is loaded", () => {
        const { mapping } = curveMapping(null);
        render(PressureCurve, { props: { mapping } });
        expect(screen.getByText("Raw")).toBeInTheDocument();
        expect(screen.getByText("Curve")).toBeInTheDocument();
        expect(screen.queryAllByRole("button")).toHaveLength(0);
    });
});
