/**
 * "Copy as template": serializes the theme currently on screen as a complete, valid theme file,
 * so authors start from something that works and edit only what they want to change.
 */
import { THEMABLE_TOKENS, THEME_SCHEMA_VERSION, toThemeKey, type ThemeBase, type TokenSpec } from "./tokens";
import { parseComputedColor, type Rgba } from "./contrast";

/** Reads the current value of a token, or `null` if it cannot be determined. */
export type TokenReader = (spec: TokenSpec) => string | null;

const hex = (n: number): string => Math.max(0, Math.min(255, Math.round(n))).toString(16).padStart(2, "0");

/** Formats a color the way theme files write them: `#rrggbb`, or `rgba()` when translucent. */
export function formatColor(c: Rgba): string {
    if (c.a >= 1) return `#${hex(c.r)}${hex(c.g)}${hex(c.b)}`;
    return `rgba(${String(Math.round(c.r))}, ${String(Math.round(c.g))}, ${String(Math.round(c.b))}, ${String(Number(c.a.toFixed(3)))})`;
}

/**
 * Builds the text of a theme file containing every themable token.
 *
 * Tokens the reader cannot resolve are left out, so the file stays valid and those tokens simply
 * inherit from `base`.
 */
export function buildThemeTemplate(read: TokenReader, base: ThemeBase, name = "My theme"): string {
    const tokens: Record<string, string> = {};
    for (const spec of THEMABLE_TOKENS) {
        const value = read(spec);
        if (value !== null && value !== "") tokens[toThemeKey(spec.name)] = value;
    }
    return `${JSON.stringify(
        { schema: THEME_SCHEMA_VERSION, metadata: { name, author: "", version: "1.0" }, base, tokens },
        null,
        2,
    )}\n`;
}

/** A reader backed by the live document: colors are resolved through a probe element. */
export function documentTokenReader(root: HTMLElement): TokenReader {
    return (spec) => {
        if (spec.kind === "color") {
            const probe = document.createElement("span");
            probe.style.cssText = `position:absolute;visibility:hidden;color:var(${spec.name})`;
            root.append(probe);
            const resolved = parseComputedColor(getComputedStyle(probe).color);
            probe.remove();
            return resolved === null ? null : formatColor(resolved);
        }
        const raw = getComputedStyle(root).getPropertyValue(spec.name).trim();
        return raw === "" ? null : raw;
    };
}
