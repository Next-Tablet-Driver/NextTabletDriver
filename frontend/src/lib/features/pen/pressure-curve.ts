/**
 * The pressure curve, as the editor draws and edits it.
 *
 * `evaluateCurve` mirrors `core::math::curve` in the engine, so the curve on screen is the one
 * the pen gets. The point rules mirror what the engine accepts: at most 32 points, sorted by x,
 * the first one at x = 0 and the last one at x = 1.
 */

export type CurveType = "Linear" | "Exponential" | "Custom";
export type CurvePoint = [number, number];

export interface CurveSettings {
    curve_type: CurveType;
    exponent: number;
    points: CurvePoint[];
}

/** The most points the engine keeps. */
export const MAX_POINTS = 32;
/** The closest two points may get on the x axis. */
export const MIN_GAP = 0.02;
/** The range of the exponent the engine accepts. */
export const EXPONENT_RANGE = { min: 0.1, max: 5 } as const;

export const DEFAULT_POINTS: CurvePoint[] = [[0, 0], [1, 1]];

const clamp01 = (value: number): number => Math.min(1, Math.max(0, value));
const round3 = (value: number): number => Math.round(value * 1000) / 1000;

function evaluateCustom(t: number, points: CurvePoint[]): number {
    if (points.length < 2) return t;
    const first = points[0];
    const last = points[points.length - 1];
    if (t <= first[0]) return first[1];
    if (t >= last[0]) return last[1];
    // The first point at or after `t`; it has a left neighbour because `t` is past the first point.
    const upper = points.findIndex((point) => point[0] >= t);
    const [x0, y0] = points[upper - 1];
    const [x1, y1] = points[upper];
    const span = x1 - x0;
    return span <= Number.EPSILON ? y1 : y0 + (y1 - y0) * ((t - x0) / span);
}

/** The pressure the pen reports for a raw pressure `t`, both from 0 to 1. */
export function evaluateCurve(t: number, curve: CurveSettings): number {
    const x = clamp01(t);
    switch (curve.curve_type) {
        case "Linear":
            return x;
        case "Exponential":
            return clamp01(x ** curve.exponent);
        case "Custom":
            return clamp01(evaluateCustom(x, curve.points));
    }
}

/** Samples the curve, for drawing. */
export function sampleCurve(curve: CurveSettings, steps = 64): CurvePoint[] {
    const samples: CurvePoint[] = [];
    for (let i = 0; i <= steps; i++) {
        const x = i / steps;
        samples.push([x, evaluateCurve(x, curve)]);
    }
    return samples;
}

/**
 * Moves a point. The first and the last point stay on the left and right edges and only move
 * up and down; the others stay between their neighbours.
 */
export function movePoint(points: CurvePoint[], index: number, x: number, y: number): CurvePoint[] {
    if (index < 0 || index >= points.length) return points;

    let newX = x;
    if (index === 0) newX = 0;
    else if (index === points.length - 1) newX = 1;
    else newX = Math.min(points[index + 1][0] - MIN_GAP, Math.max(points[index - 1][0] + MIN_GAP, x));

    return points.map((point, i): CurvePoint => (i === index ? [round3(newX), round3(clamp01(y))] : point));
}

/** Adds a point where the user clicked. Nothing changes if another point is too close or the list is full. */
export function addPoint(points: CurvePoint[], x: number, y: number): CurvePoint[] {
    if (points.length >= MAX_POINTS) return points;
    const newX = clamp01(x);
    if (points.some(([px]) => Math.abs(px - newX) < MIN_GAP)) return points;
    const next: CurvePoint[] = [...points, [round3(newX), round3(clamp01(y))]];
    return next.sort((a, b) => a[0] - b[0]);
}

/** Removes a point. The first and the last point cannot be removed. */
export function removePoint(points: CurvePoint[], index: number): CurvePoint[] {
    if (index <= 0 || index >= points.length - 1) return points;
    return points.filter((_, i) => i !== index);
}
