import { describe, it, expect } from "vitest";
import { compositeOver, contrastRatio, parseComputedColor, relativeLuminance } from "../contrast";

const white = { r: 255, g: 255, b: 255, a: 1 };
const black = { r: 0, g: 0, b: 0, a: 1 };

describe("parseComputedColor", () => {
    it("parses the forms getComputedStyle returns", () => {
        expect(parseComputedColor("rgb(204, 204, 204)")).toEqual({ r: 204, g: 204, b: 204, a: 1 });
        expect(parseComputedColor("rgba(0, 122, 204, 0.5)")).toEqual({ r: 0, g: 122, b: 204, a: 0.5 });
        expect(parseComputedColor("rgb(0 122 204 / 25%)")).toEqual({ r: 0, g: 122, b: 204, a: 0.25 });
        expect(parseComputedColor("color(srgb 0.8 0.8 0.8 / 0.4)")).toEqual({ r: 204, g: 204, b: 204, a: 0.4 });
    });

    it("returns null for anything else", () => {
        expect(parseComputedColor("")).toBeNull();
        expect(parseComputedColor("red")).toBeNull();
        expect(parseComputedColor("#fff")).toBeNull();
    });
});

describe("contrast", () => {
    it("matches the WCAG reference values", () => {
        expect(relativeLuminance(white)).toBeCloseTo(1, 5);
        expect(relativeLuminance(black)).toBe(0);
        expect(contrastRatio(white, black)).toBeCloseTo(21, 5);
        expect(contrastRatio(black, white)).toBeCloseTo(21, 5);
        expect(contrastRatio(white, white)).toBeCloseTo(1, 5);
        // #767676 on white is the classic "just passes AA" gray.
        expect(contrastRatio({ r: 118, g: 118, b: 118, a: 1 }, white)).toBeCloseTo(4.54, 1);
    });

    it("composites translucent colors over a background", () => {
        expect(compositeOver({ r: 255, g: 255, b: 255, a: 0.5 }, black)).toEqual({ r: 128, g: 128, b: 128, a: 1 });
        expect(compositeOver({ r: 10, g: 20, b: 30, a: 1 }, white)).toEqual({ r: 10, g: 20, b: 30, a: 1 });
    });
});
