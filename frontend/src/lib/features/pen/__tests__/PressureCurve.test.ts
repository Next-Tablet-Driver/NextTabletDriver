import { describe, it, expect } from "vitest";
import { render, screen, fireEvent } from "@testing-library/svelte";
import { fakeMapping, type FakeMapping } from "../../../test-support/fakeMapping";
import PressureCurve from "../PressureCurve.svelte";
import { LivePen } from "../live-pen.svelte";

interface CurveConfig {
    pressure_curve: { curve_type: string; exponent: number; points: [number, number][] };
}

function curveMapping(
    curveType: string | null,
    points: [number, number][] = [[0, 0], [0.5, 0.2], [1, 1]],
): FakeMapping<CurveConfig> {
    return fakeMapping(curveType === null ? null : { pressure_curve: { curve_type: curveType, exponent: 2, points } });
}

describe("PressureCurve", () => {
    it("offers the three curve types and highlights the current one", () => {
        const { mapping } = curveMapping("Exponential");
        render(PressureCurve, { props: { mapping } });
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

    it("draws the curve, with no selector before the config is loaded", () => {
        const { mapping } = curveMapping(null);
        render(PressureCurve, { props: { mapping } });
        expect(screen.getByLabelText("Pressure curve")).toBeInTheDocument();
        expect(screen.queryAllByRole("button")).toHaveLength(0);
    });

    it("draws a straight line for the linear curve", () => {
        const { mapping } = curveMapping("Linear");
        render(PressureCurve, { props: { mapping } });
        const path = screen.getByTestId("curve-line").getAttribute("d") ?? "";
        expect(path.startsWith("M38,202")).toBe(true);
        expect(path.endsWith("L202,38")).toBe(true);
    });

    it("shows a handle for each point of the custom curve", () => {
        const { mapping } = curveMapping("Custom");
        render(PressureCurve, { props: { mapping } });
        expect(screen.getAllByRole("slider")).toHaveLength(3);
        expect(screen.getByLabelText("Curve point 2")).toHaveAttribute("aria-valuetext", "Raw 50%, reported 20%");
    });

    it("has no handle on a curve that cannot be edited", () => {
        const { mapping } = curveMapping("Exponential");
        render(PressureCurve, { props: { mapping } });
        expect(document.querySelectorAll(".handle")).toHaveLength(0);
    });

    it("moves a point with the arrow keys", async () => {
        const { mapping, markDirty, config } = curveMapping("Custom");
        render(PressureCurve, { props: { mapping } });
        await fireEvent.keyDown(screen.getByLabelText("Curve point 2"), { key: "ArrowUp" });
        expect(config?.pressure_curve.points[1]).toEqual([0.5, 0.21]);
        expect(markDirty).toHaveBeenCalledOnce();
    });

    it("removes a middle point with Delete but keeps the ends", async () => {
        const { mapping, config } = curveMapping("Custom");
        render(PressureCurve, { props: { mapping } });
        await fireEvent.keyDown(screen.getByLabelText("Curve point 1"), { key: "Delete" });
        expect(config?.pressure_curve.points).toHaveLength(3);
        await fireEvent.keyDown(screen.getByLabelText("Curve point 2"), { key: "Delete" });
        expect(config?.pressure_curve.points).toEqual([[0, 0], [1, 1]]);
    });

    it("resets the custom curve to a straight line", async () => {
        const { mapping, config } = curveMapping("Custom");
        render(PressureCurve, { props: { mapping } });
        await fireEvent.click(screen.getByRole("button", { name: "Reset curve" }));
        expect(config?.pressure_curve.points).toEqual([[0, 0], [1, 1]]);
    });

    describe("live pen", () => {
        function pen(status: string, pressure: number): LivePen {
            const live = new LivePen();
            live.update({ status, x: 0, y: 0, pressure, max_pressure: 1000, hover_distance: 0, tilt_x: 0, tilt_y: 0 });
            return live;
        }

        it("shows no dot while the pen is away", () => {
            const { mapping } = curveMapping("Linear");
            render(PressureCurve, { props: { mapping, live: pen("OutOfRange", 0) } });
            expect(screen.queryByTestId("live-dot")).toBeNull();
            expect(screen.getByTestId("live-raw")).toHaveTextContent("—");
        });

        it("puts the dot on the curve at the pressure of the pen", () => {
            const { mapping } = curveMapping("Linear");
            render(PressureCurve, { props: { mapping, live: pen("Contact", 400) } });
            expect(screen.getByTestId("live-dot")).toBeInTheDocument();
            expect(screen.getByTestId("live-raw")).toHaveTextContent("40%");
            expect(screen.getByTestId("live-output")).toHaveTextContent("40%");
        });

        it("reports what the curve makes of the pressure", () => {
            const { mapping } = curveMapping("Custom", [[0, 0], [0.5, 0.2], [1, 1]]);
            render(PressureCurve, { props: { mapping, live: pen("Contact", 500) } });
            expect(screen.getByTestId("live-raw")).toHaveTextContent("50%");
            expect(screen.getByTestId("live-output")).toHaveTextContent("20%");
        });

        it("says whether the pen touches or hovers", () => {
            const { mapping } = curveMapping("Linear");
            const { unmount } = render(PressureCurve, { props: { mapping, live: pen("Hover", 0) } });
            expect(screen.getByText("Hovering")).toBeInTheDocument();
            unmount();
            render(PressureCurve, { props: { mapping, live: pen("Contact", 100) } });
            expect(screen.getByText("Touching")).toBeInTheDocument();
        });
    });

    it("sets the exponent of the exponential curve", async () => {
        const { mapping, markDirty, config } = curveMapping("Exponential");
        render(PressureCurve, { props: { mapping } });
        await fireEvent.input(screen.getByRole("slider"), { target: { value: "3" } });
        expect(config?.pressure_curve.exponent).toBe(3);
        expect(markDirty).toHaveBeenCalledOnce();
    });
});
