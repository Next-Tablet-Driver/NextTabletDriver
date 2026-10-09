/// <reference types="node" />
/**
 * The theme documentation, JSON Schema and example themes are part of the product: these tests
 * keep them in sync with the code.
 */
import { describe, it, expect } from "vitest";
import fs from "node:fs";
import path from "node:path";
import { THEMABLE_TOKENS, toThemeKey } from "../tokens";
import { themeJsonSchemaText } from "../json-schema";
import { parseTheme } from "../validate";
import { BUILTIN_THEMES } from "../apply";

// Vitest runs from the `frontend` directory.
const REPO = path.resolve(process.cwd(), "..");
const DOCS = path.join(REPO, "docs");

describe("theme documentation", () => {
    it("documents every themable token in THEMES.md", () => {
        const doc = fs.readFileSync(path.join(DOCS, "THEMES.md"), "utf8");
        const missing = THEMABLE_TOKENS.map((t) => toThemeKey(t.name)).filter((key) => !doc.includes(`\`${key}\``));
        expect(missing).toEqual([]);
    });

    it("lists every built-in theme in THEMES.md", () => {
        const doc = fs.readFileSync(path.join(DOCS, "THEMES.md"), "utf8");
        const missing = BUILTIN_THEMES.filter((t) => !doc.includes(`\`${t.configValue}\``)).map((t) => t.configValue);
        expect(missing).toEqual([]);
    });

    it("ships a JSON Schema that matches the token contract (run `npm run test -- -u` to refresh)", async () => {
        await expect(themeJsonSchemaText()).toMatchFileSnapshot(path.join(DOCS, "themes", "theme.schema.json"));
    });

    const examples = fs.readdirSync(path.join(DOCS, "themes", "examples")).filter((f) => f.endsWith(".json"));

    it("has example themes", () => {
        expect(examples.length).toBeGreaterThan(0);
    });

    it.each(examples)("example %s is a valid theme without warnings", (file) => {
        const result = parseTheme(fs.readFileSync(path.join(DOCS, "themes", "examples", file), "utf8"));
        expect(result.ok ? result.warnings : result.errors).toEqual([]);
        expect(result.ok).toBe(true);
    });
});
