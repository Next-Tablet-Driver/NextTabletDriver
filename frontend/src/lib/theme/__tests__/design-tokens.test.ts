/// <reference types="node" />
/**
 * Permanent guard rails for the design system.
 *
 * These tests read the source tree and fail when someone reintroduces what the design system
 * removed: an undefined token, a hard-coded color, or a theming contract that no longer matches
 * the stylesheet.
 */
import { describe, it, expect } from "vitest";
import fs from "node:fs";
import path from "node:path";
import { THEMABLE_TOKENS, type TokenGroup } from "../tokens";
import { BUILTIN_THEMES } from "../apply";

// Vitest runs from the `frontend` directory.
const SRC = path.resolve(process.cwd(), "src");

function walk(dir: string, out: string[] = []): string[] {
    for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
        const full = path.join(dir, entry.name);
        if (entry.isDirectory()) walk(full, out);
        else out.push(full);
    }
    return out;
}

const rel = (file: string): string => path.relative(SRC, file).split(path.sep).join("/");
const read = (file: string): string => fs.readFileSync(file, "utf8");

const ALL_FILES = walk(SRC);
const STYLED = ALL_FILES.filter((f) => /\.(svelte|css|ts)$/.test(f));
const tokensCss = read(path.join(SRC, "styles/tokens.css"));

/** Source files that are allowed to contain raw colors. */
function mayContainColors(file: string): boolean {
    const r = rel(file);
    return (
        r.startsWith("styles/") || // the token definitions themselves
        r.startsWith("dev/") || // fixtures and dev tooling
        r.startsWith("lib/theme/") || // the theme engine and its test data
        r.includes("/__tests__/")
    );
}

/** Every custom property declared anywhere (CSS declarations and Svelte `style:--x` directives). */
function declaredCustomProperties(): Set<string> {
    const declared = new Set<string>();
    for (const file of STYLED) {
        const text = read(file);
        for (const m of text.matchAll(/(?<![\w-])(--[a-z0-9-]+)\s*:/g)) declared.add(m[1]);
        for (const m of text.matchAll(/style:(--[a-z0-9-]+)/g)) declared.add(m[1]);
    }
    return declared;
}

/** Every `var(--x)` reference in non-test sources. */
function usedCustomProperties(): { file: string; name: string }[] {
    const used: { file: string; name: string }[] = [];
    // Dev tooling builds token names dynamically (`var(--${name})`), which cannot be checked statically.
    for (const file of STYLED.filter((f) => !rel(f).includes("/__tests__/") && !rel(f).startsWith("dev/"))) {
        for (const m of read(file).matchAll(/var\(\s*(--[a-z0-9-]+)/g)) used.push({ file: rel(file), name: m[1] });
    }
    return used;
}

describe("design tokens", () => {
    it("declares every token of the theming contract in tokens.css", () => {
        const missing = THEMABLE_TOKENS.filter((t) => !new RegExp(`^\\s*${t.name}\\s*:`, "m").test(tokensCss)).map(
            (t) => t.name,
        );
        expect(missing).toEqual([]);
    });

    it("has no duplicate or malformed names in the contract", () => {
        const names = THEMABLE_TOKENS.map((t) => t.name);
        expect(new Set(names).size).toBe(names.length);
        for (const name of names) expect(name).toMatch(/^--[a-z0-9]+(?:-[a-z0-9]+)*$/);
    });

    it("never uses a custom property that is declared nowhere", () => {
        const declared = declaredCustomProperties();
        const undefinedUses = usedCustomProperties()
            .filter(({ name }) => !declared.has(name))
            .map(({ file, name }) => `${file}: ${name}`);
        expect([...new Set(undefinedUses)]).toEqual([]);
    });

    it("has no hard-coded colors outside the token and theme files", () => {
        const COLOR = /#[0-9a-fA-F]{3,8}\b|\brgba?\(|\bhsla?\(/;
        const offenders: string[] = [];
        for (const file of STYLED) {
            if (mayContainColors(file)) continue;
            // `url(...)` data URIs may legitimately embed a color.
            const text = read(file).replace(/url\((['"]?)data:[^)]*\)/g, "url()");
            text.split("\n").forEach((line, i) => {
                if (COLOR.test(line)) offenders.push(`${rel(file)}:${String(i + 1)}: ${line.trim().slice(0, 80)}`);
            });
        }
        expect(offenders).toEqual([]);
    });

    it("has no `!important` overrides in components", () => {
        const offenders: string[] = [];
        for (const file of STYLED) {
            if (rel(file).startsWith("dev/") || rel(file).includes("/__tests__/")) continue;
            if (read(file).includes("!important")) offenders.push(rel(file));
        }
        expect(offenders).toEqual([]);
    });

    describe("built-in themes", () => {
        /** Groups every dark/light theme must define in full, so none inherits another brightness. */
        const CORE_GROUPS: readonly TokenGroup[] = ["surface", "text", "accent", "border", "status", "overlay", "input"];
        /** Core tokens that are derived from other tokens and need no explicit value. */
        const DERIVED = new Set(["--border-hover", "--bg-tab-active"]);
        const themesCss = read(path.join(SRC, "styles/themes.css"));

        function declaredBy(themeId: string): Set<string> {
            const block = new RegExp(String.raw`:root\.theme-${themeId}\s*\{([^}]*)\}`).exec(themesCss);
            const names = new Set<string>();
            for (const m of (block?.[1] ?? "").matchAll(/(--[a-z0-9-]+|color-scheme)\s*:/g)) names.add(m[1]);
            return names;
        }

        const core = THEMABLE_TOKENS.filter((t) => CORE_GROUPS.includes(t.group) && !DERIVED.has(t.name));

        it.each(BUILTIN_THEMES.filter((t) => t.id !== "dark"))("$id defines every core token and its color-scheme", (theme) => {
            const declared = declaredBy(theme.id);
            expect(core.map((t) => t.name).filter((name) => !declared.has(name))).toEqual([]);
            expect(declared.has("color-scheme")).toBe(true);
        });

        it("has a CSS block for every built-in theme and no block for an unknown one", () => {
            const inCss = [...themesCss.matchAll(/:root\.theme-([a-z0-9]+)\s*\{/g)].map((m) => m[1]).sort();
            const registered = BUILTIN_THEMES.filter((t) => t.id !== "dark").map((t) => t.id).sort();
            expect(inCss).toEqual(registered);
        });

        it("keeps `dark` as the default declared in tokens.css", () => {
            const missing = core.filter((t) => !new RegExp(String.raw`^\s*${t.name}\s*:`, "m").test(tokensCss)).map((t) => t.name);
            expect(missing).toEqual([]);
        });
    });
});
