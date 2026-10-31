import { describe, it, expect } from "vitest";
import {
    MAX_POINTS, MIN_GAP, addPoint, evaluateCurve, movePoint, removePoint, sampleCurve,
    type CurvePoint, type CurveSettings,
} from "../pressure-curve";

const linear: CurveSettings = { curve_type: "Linear", exponent: 2, points: [[0, 0], [1, 1]] };
const custom = (points: CurvePoint[]): CurveSettings => ({ curve_type: "Custom", exponent: 2, points });

describe("evaluateCurve", () => {
    it("leaves the pressure alone on the linear curve", () => {
        expect(evaluateCurve(0.3, linear)).toBeCloseTo(0.3);
    });

    it("raises the pressure to the exponent", () => {
        expect(evaluateCurve(0.5, { ...linear, curve_type: "Exponential", exponent: 2 })).toBeCloseTo(0.25);
    });

    it("interpolates between the points of a custom curve", () => {
        const curve = custom([[0, 0], [0.5, 0.9], [1, 1]]);
        expect(evaluateCurve(0.25, curve)).toBeCloseTo(0.45);
        expect(evaluateCurve(0.5, curve)).toBeCloseTo(0.9);
    });

    it("clamps the input and the output", () => {
        expect(evaluateCurve(-1, linear)).toBe(0);
        expect(evaluateCurve(2, linear)).toBe(1);
    });

    it("falls back to the identity with fewer than two points", () => {
        expect(evaluateCurve(0.3, custom([[0.5, 0.9]]))).toBeCloseTo(0.3);
    });
});

describe("sampleCurve", () => {
    it("covers the whole range, ends included", () => {
        const samples = sampleCurve(linear, 10);
        expect(samples).toHaveLength(11);
        expect(samples[0]).toEqual([0, 0]);
        expect(samples[10]).toEqual([1, 1]);
    });
});

describe("movePoint", () => {
    const points: CurvePoint[] = [[0, 0], [0.5, 0.5], [1, 1]];

    it("moves a middle point", () => {
        expect(movePoint(points, 1, 0.4, 0.7)).toEqual([[0, 0], [0.4, 0.7], [1, 1]]);
    });

    it("keeps a middle point between its neighbours", () => {
        expect(movePoint(points, 1, -1, 0.5)[1]).toEqual([MIN_GAP, 0.5]);
        expect(movePoint(points, 1, 2, 0.5)[1]).toEqual([1 - MIN_GAP, 0.5]);
    });

    it("keeps the ends on the edges, only moving them up and down", () => {
        expect(movePoint(points, 0, 0.3, 0.2)[0]).toEqual([0, 0.2]);
        expect(movePoint(points, 2, 0.3, 0.8)[2]).toEqual([1, 0.8]);
    });

    it("keeps the height between 0 and 1", () => {
        expect(movePoint(points, 1, 0.5, 3)[1]).toEqual([0.5, 1]);
        expect(movePoint(points, 1, 0.5, -3)[1]).toEqual([0.5, 0]);
    });

    it("does not change the original list", () => {
        movePoint(points, 1, 0.4, 0.7);
        expect(points[1]).toEqual([0.5, 0.5]);
    });

    it("ignores a point that does not exist", () => {
        expect(movePoint(points, 9, 0.4, 0.7)).toBe(points);
    });
});

describe("addPoint", () => {
    it("inserts the point in order", () => {
        expect(addPoint([[0, 0], [1, 1]], 0.3, 0.6)).toEqual([[0, 0], [0.3, 0.6], [1, 1]]);
        expect(addPoint([[0, 0], [0.8, 0.8], [1, 1]], 0.3, 0.6)[1]).toEqual([0.3, 0.6]);
    });

    it("refuses a point too close to another one", () => {
        const points: CurvePoint[] = [[0, 0], [1, 1]];
        expect(addPoint(points, 0.005, 0.5)).toBe(points);
    });

    it("refuses a point once the list is full", () => {
        const full: CurvePoint[] = Array.from({ length: MAX_POINTS }, (_, i): CurvePoint => [i / (MAX_POINTS - 1), 0.5]);
        expect(addPoint(full, 0.0123, 0.5)).toBe(full);
    });
});

describe("removePoint", () => {
    const points: CurvePoint[] = [[0, 0], [0.5, 0.5], [1, 1]];

    it("removes a middle point", () => {
        expect(removePoint(points, 1)).toEqual([[0, 0], [1, 1]]);
    });

    it("keeps the first and the last point", () => {
        expect(removePoint(points, 0)).toBe(points);
        expect(removePoint(points, 2)).toBe(points);
    });
});
