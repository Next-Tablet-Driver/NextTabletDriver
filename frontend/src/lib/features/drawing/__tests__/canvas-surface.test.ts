import { describe, it, expect } from "vitest";
import { canvasPoint, needsResize } from "../canvas-surface";

const canvas = (width: number, height: number, box: { left: number; top: number; width: number; height: number }): Parameters<typeof canvasPoint>[0] => ({
    width,
    height,
    getBoundingClientRect: () => ({ ...box, right: box.left + box.width, bottom: box.top + box.height, x: box.left, y: box.top, toJSON: () => ({}) }),
});

describe("canvasPoint", () => {
    it("is relative to the corner of the canvas", () => {
        const surface = canvas(400, 200, { left: 100, top: 50, width: 400, height: 200 });
        expect(canvasPoint(surface, 150, 80)).toEqual({ x: 50, y: 30 });
    });

    it("follows the scale when the canvas is drawn at another size", () => {
        const surface = canvas(400, 200, { left: 0, top: 0, width: 800, height: 400 });
        expect(canvasPoint(surface, 200, 100)).toEqual({ x: 100, y: 50 });
    });

    it("scales each axis on its own", () => {
        const surface = canvas(400, 200, { left: 0, top: 0, width: 400, height: 400 });
        expect(canvasPoint(surface, 100, 100)).toEqual({ x: 100, y: 50 });
    });

    it("does not divide by an empty box", () => {
        const surface = canvas(400, 200, { left: 0, top: 0, width: 0, height: 0 });
        expect(canvasPoint(surface, 10, 20)).toEqual({ x: 10, y: 20 });
    });
});

describe("needsResize", () => {
    it("is true when either side differs", () => {
        expect(needsResize({ width: 300, height: 150 }, 400, 150)).toBe(true);
        expect(needsResize({ width: 300, height: 150 }, 300, 200)).toBe(true);
    });

    it("is false when the size already matches", () => {
        expect(needsResize({ width: 300, height: 150 }, 300, 150)).toBe(false);
    });

    it("is false for an empty size, such as a page that is not shown", () => {
        expect(needsResize({ width: 300, height: 150 }, 0, 0)).toBe(false);
        expect(needsResize({ width: 300, height: 150 }, 400, 0)).toBe(false);
    });
});
