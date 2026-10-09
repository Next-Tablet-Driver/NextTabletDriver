import { describe, it, expect } from "vitest";
import { buildThemeTemplate, formatColor, type TokenReader } from "../template";
import { THEMABLE_TOKENS, toThemeKey } from "../tokens";
import { parseTheme } from "../validate";

describe("formatColor", () => {
    it("writes opaque colors as hex and translucent ones as rgba()", () => {
        expect(formatColor({ r: 255, g: 0, b: 127, a: 1 })).toBe("#ff007f");
        expect(formatColor({ r: 0, g: 0, b: 0, a: 1 })).toBe("#000000");
        expect(formatColor({ r: 255, g: 255, b: 255, a: 0.15 })).toBe("rgba(255, 255, 255, 0.15)");
        expect(formatColor({ r: 255.4, g: 0, b: 0, a: 0.3333333 })).toBe("rgba(255, 0, 0, 0.333)");
    });
});

/** A reader that returns a plausible valid value for every token kind. */
const validReader: TokenReader = (spec) => {
    switch (spec.kind) {
        case "color":
            return "#336699";
        case "rgb-triplet":
            return "255 255 255";
        case "length":
            return "4px";
        case "number":
            return "0.5";
        case "font-family":
            return "'Segoe UI', sans-serif";
    }
};

describe("buildThemeTemplate", () => {
    it("produces a file that the validator accepts, with every themable token", () => {
        const result = parseTheme(buildThemeTemplate(validReader, "light", "Starter"));
        expect(result.ok).toBe(true);
        if (!result.ok) return;
        expect(result.theme.base).toBe("light");
        expect(result.theme.metadata.name).toBe("Starter");
        expect(Object.keys(result.theme.tokens).sort()).toEqual(THEMABLE_TOKENS.map((t) => t.name).sort());
        expect(result.warnings).toEqual([]);
    });

    it("leaves out tokens that cannot be read instead of writing invalid values", () => {
        const partial: TokenReader = (spec) => (spec.name === "--accent" ? "#ff007f" : null);
        const result = parseTheme(buildThemeTemplate(partial, "dark"));
        expect(result.ok && result.theme.tokens).toEqual({ "--accent": "#ff007f" });
    });

    it("uses the theme-file key (no leading dashes)", () => {
        const text = buildThemeTemplate(validReader, "dark");
        const tokens = (JSON.parse(text) as { tokens: Record<string, string> }).tokens;
        expect(Object.keys(tokens)).toContain(toThemeKey("--bg-app"));
        expect(Object.keys(tokens).some((k) => k.startsWith("--"))).toBe(false);
    });
});
