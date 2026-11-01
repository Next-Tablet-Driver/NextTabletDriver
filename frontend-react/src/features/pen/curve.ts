import type { Config } from "@/infrastructure/tauri/commands";

export type PressureCurve = Config["pressure_curve"];
export type CurveType = PressureCurve["curve_type"];
export type CurvePoint = [number, number];

/** `null` is how the backend sends a number it cannot represent. */
const finite = (value: number | null): number => value ?? 0;

/** The control points of a custom curve, with `null` read as 0. */
export function pointsOf(curve: PressureCurve): CurvePoint[] {
  return curve.points.map(([x, y]) => [finite(x), finite(y)]);
}

const clamp01 = (value: number): number => Math.max(0, Math.min(1, value));

/** The same arithmetic as the driver (`ntd-core::math::curve`), to draw what it will do. */
export function evaluateCurve(input: number, curve: PressureCurve): number {
  const t = clamp01(input);
  switch (curve.curve_type) {
    case "Linear":
      return t;
    case "Exponential":
      return clamp01(t ** finite(curve.exponent));
    case "Custom":
      return clamp01(evaluateCustom(t, pointsOf(curve)));
  }
}

function evaluateCustom(t: number, points: readonly CurvePoint[]): number {
  const first = points[0];
  const last = points.at(-1);
  if (first === undefined || last === undefined || points.length < 2) {
    return t;
  }
  if (t <= first[0]) {
    return first[1];
  }
  if (t >= last[0]) {
    return last[1];
  }
  for (let i = 1; i < points.length; i++) {
    const a = points[i - 1];
    const b = points[i];
    if (a === undefined || b === undefined || t < a[0] || t > b[0]) {
      continue;
    }
    const span = b[0] - a[0];
    return span <= Number.EPSILON ? b[1] : a[1] + (b[1] - a[1]) * ((t - a[0]) / span);
  }
  return t;
}

/** A polyline of the curve for an SVG `points` attribute, in a `size` by `size` box. */
export function curvePath(curve: PressureCurve, size: number, steps = 64): string {
  const coordinates: string[] = [];
  for (let i = 0; i <= steps; i++) {
    const t = i / steps;
    coordinates.push(
      `${(t * size).toFixed(2)},${((1 - evaluateCurve(t, curve)) * size).toFixed(2)}`,
    );
  }
  return coordinates.join(" ");
}

/** Sorts the points by `x` and pins the first to 0 and the last to 1, as the driver expects. */
export function normalizePoints(points: readonly CurvePoint[]): CurvePoint[] {
  const sorted = points.map(([x, y]): CurvePoint => [clamp01(x), clamp01(y)]);
  sorted.sort((a, b) => a[0] - b[0]);
  const first = sorted[0];
  const last = sorted.at(-1);
  if (first !== undefined) {
    first[0] = 0;
  }
  if (last !== undefined && sorted.length > 1) {
    last[0] = 1;
  }
  return sorted;
}

/** The default control points of a new custom curve: the identity. */
export const IDENTITY_POINTS: readonly CurvePoint[] = [
  [0, 0],
  [1, 1],
];
