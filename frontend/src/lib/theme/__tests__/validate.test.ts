import { describe, it, expect } from "vitest";
import { LIMITS, checkTokenValue, parseTheme } from "../validate";
import { getTokenSpec, type TokenSpec } from "../tokens";

function spec(name: string): TokenSpec {
    const found = getTokenSpec(name);
    if (found === undefined) throw new Error(`Unknown token ${name}`);
    return found;
}

function theme(overrides: Record<string, unknown> = {}, tokens: Record<string, unknown> = { accent: "#ff007f" }): string {
    return JSON.stringify({
        schema: 1,
        metadata: { name: "Test theme", author: "Me", version: "1.0" },
        base: "dark",
        tokens,
        ...overrides,
    });
}

function errorsOf(text: string): string[] {
    const result = parseTheme(text);
    return result.ok ? [] : result.errors.map((e) => `${e.path}: ${e.message}`);
}

describe("parseTheme", () => {
    it("accepts a valid theme and prefixes token names with --", () => {
        const result = parseTheme(theme({}, { accent: "#ff007f", "bg-app": "rgb(16, 16, 24)", "radius-md": "6px" }));
        expect(result.ok).toBe(true);
        if (!result.ok) return;
        expect(result.theme.metadata.name).toBe("Test theme");
        expect(result.theme.base).toBe("dark");
        expect(result.theme.tokens).toEqual({
            "--accent": "#ff007f",
            "--bg-app": "rgb(16, 16, 24)",
            "--radius-md": "6px",
        });
    });

    it("rejects input that is not JSON, not an object, or too large", () => {
        expect(errorsOf("{ nope")[0]).toMatch(/not valid JSON/);
        expect(errorsOf("[]")[0]).toMatch(/top level must be an object/);
        expect(errorsOf(" ".repeat(LIMITS.maxFileLength + 1))[0]).toMatch(/larger than/);
    });

    it("requires the supported schema version", () => {
        expect(errorsOf(theme({ schema: 2 }))).toContainEqual(expect.stringContaining("unsupported schema version (2)"));
        expect(errorsOf(theme({ schema: undefined }))).toContainEqual(expect.stringContaining("(missing)"));
    });

    it("requires a non-empty metadata.name and strips control characters", () => {
        expect(errorsOf(theme({ metadata: {} }))).toContainEqual("metadata.name: is required");
        expect(errorsOf(theme({ metadata: { name: "  " } }))).toContainEqual("metadata.name: must not be empty");
        expect(errorsOf(theme({ metadata: undefined }))).toContainEqual(expect.stringContaining("metadata:"));

        const result = parseTheme(theme({ metadata: { name: "Line1\nLine2\u0000" } }));
        expect(result.ok && result.theme.metadata.name).toBe("Line1 Line2");
    });

    it("validates base and defaults to dark with a warning", () => {
        expect(errorsOf(theme({ base: "sepia" }))).toContainEqual('base: must be "dark" or "light"');
        const result = parseTheme(theme({ base: undefined }));
        expect(result.ok).toBe(true);
        if (result.ok) {
            expect(result.theme.base).toBe("dark");
            expect(result.warnings.map((w) => w.path)).toContain("base");
        }
        const light = parseTheme(theme({ base: "light" }));
        expect(light.ok && light.theme.base).toBe("light");
    });

    it("ignores unknown tokens with a warning instead of failing (forward compatible)", () => {
        const result = parseTheme(theme({}, { accent: "#fff", "future-token": "#000", "spacing-md": "99px" }));
        expect(result.ok).toBe(true);
        if (!result.ok) return;
        expect(Object.keys(result.theme.tokens)).toEqual(["--accent"]);
        expect(result.warnings.map((w) => w.path).sort()).toEqual(["tokens.future-token", "tokens.spacing-md"]);
    });

    it("reports every invalid token, not just the first", () => {
        const errors = errorsOf(theme({}, { accent: "red-ish", "bg-app": 12, "radius-md": "100px" }));
        expect(errors).toHaveLength(3);
    });

    it("limits the number of tokens", () => {
        const many: Record<string, string> = {};
        for (let i = 0; i <= LIMITS.maxTokens; i++) many[`token-${String(i)}`] = "#fff";
        expect(errorsOf(theme({}, many))).toContainEqual(expect.stringContaining("more than"));
    });

    it("does not let __proto__ keys do anything", () => {
        const text = '{"schema":1,"metadata":{"name":"x"},"base":"dark","tokens":{"__proto__":"#fff","accent":"#fff"}}';
        const result = parseTheme(text);
        expect(result.ok).toBe(true);
        expect(({} as Record<string, unknown>).accent).toBeUndefined();
        if (result.ok) expect(Object.keys(result.theme.tokens)).toEqual(["--accent"]);
    });
});

describe("value grammars", () => {
    const color = spec("--accent");
    const triplet = spec("--overlay-rgb");
    const length = spec("--radius-md");
    const number = spec("--disabled-opacity");
    const font = spec("--font-sans");

    it.each([
        "#fff", "#FFFF", "#ff007f", "#ff007f80", "transparent",
        "rgb(255, 0, 127)", "rgba(255, 0, 127, 0.5)", "rgb(255 0 127 / 0.5)", "hsl(210, 50%, 40%)", "hsla(210deg, 50%, 40%, 0.3)",
    ])("accepts the color %s", (value) => {
        expect(checkTokenValue(color, value)).toBeNull();
    });

    it.each([
        "", "red", "#ff", "#ggg", "#fffffffff", "rgb(0,0,0", "rgb(var(--x), 0, 0)", "url(http://evil.example/x.png)",
        "#fff; background: url(x)", "#fff } body { display: none", "rgb(0 0 0) !important", "expression(alert(1))",
        "#fff /* hi */", "\\72 ed", "@import 'x'", "var(--accent)", "calc(1px + 2px)",
    ])("rejects the color %j", (value) => {
        expect(checkTokenValue(color, value)).not.toBeNull();
    });

    it("validates RGB triplets", () => {
        expect(checkTokenValue(triplet, "255 255 255")).toBeNull();
        expect(checkTokenValue(triplet, "0 0 0")).toBeNull();
        expect(checkTokenValue(triplet, "256 0 0")).not.toBeNull();
        expect(checkTokenValue(triplet, "255,255,255")).not.toBeNull();
        expect(checkTokenValue(triplet, "255 255")).not.toBeNull();
    });

    it("validates and bounds lengths", () => {
        expect(checkTokenValue(length, "0px")).toBeNull();
        expect(checkTokenValue(length, "6px")).toBeNull();
        expect(checkTokenValue(length, "0.5rem")).toBeNull();
        expect(checkTokenValue(length, "32px")).toBeNull();
        expect(checkTokenValue(length, "33px")).not.toBeNull();
        expect(checkTokenValue(length, "-1px")).not.toBeNull();
        expect(checkTokenValue(length, "10%")).not.toBeNull();
        expect(checkTokenValue(length, "6")).not.toBeNull();
    });

    it("validates bounded numbers", () => {
        expect(checkTokenValue(number, "0.5")).toBeNull();
        expect(checkTokenValue(number, "1")).toBeNull();
        expect(checkTokenValue(number, "1.5")).not.toBeNull();
        expect(checkTokenValue(number, "abc")).not.toBeNull();
    });

    it("accepts plain font stacks and rejects anything that is not a font name", () => {
        expect(checkTokenValue(font, "'Segoe UI', Tahoma, sans-serif")).toBeNull();
        expect(checkTokenValue(font, '"Fira Code", monospace')).toBeNull();
        expect(checkTokenValue(font, "Inter")).toBeNull();
        expect(checkTokenValue(font, "Inter, url(x)")).not.toBeNull();
        expect(checkTokenValue(font, "Inter; color: red")).not.toBeNull();
        expect(checkTokenValue(font, "Inter } body {")).not.toBeNull();
        expect(checkTokenValue(font, "a, b, c, d, e, f, g, h, i")).not.toBeNull();
    });

    it("rejects values longer than the limit", () => {
        expect(checkTokenValue(color, `#${"f".repeat(LIMITS.maxValueLength)}`)).not.toBeNull();
    });
});
