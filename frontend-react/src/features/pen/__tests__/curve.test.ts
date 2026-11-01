import { describe, expect, it } from "vitest";
import {
  curvePath,
  evaluateCurve,
  normalizePoints,
  type PressureCurve,
} from "@/features/pen/curve";

const linear: PressureCurve = { curve_type: "Linear", exponent: 2, points: [] };

describe("evaluateCurve", () => {
  it("is the identity for a linear curve, clamped to [0, 1]", () => {
    expect(evaluateCurve(0.3, linear)).toBeCloseTo(0.3);
    expect(evaluateCurve(5, linear)).toBe(1);
    expect(evaluateCurve(-1, linear)).toBe(0);
  });

  it("raises the input to the exponent", () => {
    const curve: PressureCurve = { ...linear, curve_type: "Exponential", exponent: 2 };
    expect(evaluateCurve(0.5, curve)).toBeCloseTo(0.25);
    expect(evaluateCurve(1, curve)).toBe(1);
  });

  it("interpolates between the control points of a custom curve", () => {
    const curve: PressureCurve = {
      curve_type: "Custom",
      exponent: 1,
      points: [
        [0, 0],
        [0.5, 0.8],
        [1, 1],
      ],
    };
    expect(evaluateCurve(0.5, curve)).toBeCloseTo(0.8);
    expect(evaluateCurve(0.25, curve)).toBeCloseTo(0.4);
    expect(evaluateCurve(0.75, curve)).toBeCloseTo(0.9);
  });

  it("falls back to the identity with fewer than two points", () => {
    const curve: PressureCurve = { curve_type: "Custom", exponent: 1, points: [[0, 0]] };
    expect(evaluateCurve(0.4, curve)).toBeCloseTo(0.4);
  });

  it("reads null numbers as 0", () => {
    const curve: PressureCurve = { curve_type: "Exponential", exponent: null, points: [] };
    expect(evaluateCurve(0.5, curve)).toBe(1);
  });
});

describe("curvePath", () => {
  it("starts bottom left and ends top right for a linear curve", () => {
    const points = curvePath(linear, 100, 4).split(" ");
    expect(points[0]).toBe("0.00,100.00");
    expect(points.at(-1)).toBe("100.00,0.00");
  });
});

describe("normalizePoints", () => {
  it("sorts the points and pins the ends to 0 and 1", () => {
    expect(
      normalizePoints([
        [0.9, 0.9],
        [0.1, 0.2],
        [0.5, 0.5],
      ]),
    ).toEqual([
      [0, 0.2],
      [0.5, 0.5],
      [1, 0.9],
    ]);
  });

  it("keeps the values in [0, 1]", () => {
    expect(
      normalizePoints([
        [0, -3],
        [1, 9],
      ]),
    ).toEqual([
      [0, 0],
      [1, 1],
    ]);
  });
});
