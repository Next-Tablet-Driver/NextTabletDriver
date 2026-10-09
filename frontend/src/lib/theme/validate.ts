/**
 * Validation of user theme files.
 *
 * A theme file is untrusted input: it comes from the user's disk, usually downloaded from
 * somewhere. It is therefore *data only*: a fixed set of token names, each with a strict value
 * grammar. Nothing from the file is ever concatenated into a stylesheet; values are later set
 * one by one with `style.setProperty`, and only after passing the checks below.
 *
 * File format (version 1):
 *
 *     {
 *       "schema": 1,
 *       "metadata": { "name": "My theme", "author": "Me", "version": "1.0" },
 *       "base": "dark",
 *       "tokens": { "accent": "#ff007f", "bg-app": "#101018" }
 *     }
 */
import {
    THEME_BASES,
    THEME_SCHEMA_VERSION,
    getTokenSpec,
    type ThemeBase,
    type TokenSpec,
} from "./tokens";

/** Hard limits, so a hostile or broken file cannot make the app do unbounded work. */
export const LIMITS = {
    /** Maximum size of a theme file, in characters. */
    maxFileLength: 64 * 1024,
    /** Maximum number of `tokens` entries. */
    maxTokens: 200,
    /** Maximum length of one token value. */
    maxValueLength: 200,
    /** Maximum length of metadata strings. */
    maxMetadataLength: 200,
    /** Maximum entries in a font stack. */
    maxFontFamilies: 8,
} as const;

export interface ThemeMetadata {
    name: string;
    author?: string;
    version?: string;
    description?: string;
}

export interface ThemeDefinition {
    schema: typeof THEME_SCHEMA_VERSION;
    metadata: ThemeMetadata;
    base: ThemeBase;
    /** CSS custom property name (with `--`) -> validated value. */
    tokens: Record<string, string>;
}

export interface ValidationIssue {
    severity: "error" | "warning";
    /** Where in the file the problem is, e.g. `tokens.accent`. */
    path: string;
    message: string;
}

export type ParseResult =
    | { ok: true; theme: ThemeDefinition; warnings: ValidationIssue[] }
    | { ok: false; errors: ValidationIssue[] };

// --- value grammars -------------------------------------------------------------------------

const HEX_COLOR = /^#(?:[0-9a-f]{3,4}|[0-9a-f]{6}|[0-9a-f]{8})$/i;
// Functional colors: only digits, separators and the channel units. No nested functions.
const FUNCTIONAL_COLOR = /^(?:rgb|rgba|hsl|hsla)\(\s*[0-9.]+(?:%|deg)?(?:\s*[,\s/]\s*[0-9.]+%?){2,3}\s*\)$/i;
const RGB_TRIPLET = /^(\d{1,3})\s+(\d{1,3})\s+(\d{1,3})$/;
const LENGTH = /^(\d+(?:\.\d+)?)(px|rem|em)$/;
const NUMBER = /^\d+(?:\.\d+)?$/;
const FONT_BARE = /^[A-Za-z][A-Za-z0-9_-]*(?: [A-Za-z0-9_-]+)*$/;
const FONT_QUOTED = /^(["'])[A-Za-z0-9 ._-]{1,48}\1$/;

const GENERIC_FONTS = new Set([
    "serif",
    "sans-serif",
    "monospace",
    "cursive",
    "fantasy",
    "system-ui",
    "ui-sans-serif",
    "ui-serif",
    "ui-monospace",
    "ui-rounded",
]);

function isValidColor(value: string): boolean {
    if (value.toLowerCase() === "transparent") return true;
    if (HEX_COLOR.test(value)) return true;
    return FUNCTIONAL_COLOR.test(value);
}

function checkTriplet(value: string): string | null {
    const match = RGB_TRIPLET.exec(value);
    if (match === null) return "expected three channels between 0 and 255, e.g. `255 255 255`";
    const channels = [match[1], match[2], match[3]].map(Number);
    return channels.every((c) => c <= 255) ? null : "channels must be between 0 and 255";
}

function checkLength(value: string, spec: TokenSpec): string | null {
    const match = LENGTH.exec(value);
    if (match === null) return "expected a length such as `6px`";
    const amount = Number(match[1]);
    const px = match[2] === "px" ? amount : amount * 16;
    const max = spec.max ?? 64;
    return px <= max ? null : `must be at most ${String(max)}px`;
}

function checkNumber(value: string, spec: TokenSpec): string | null {
    if (!NUMBER.test(value)) return "expected a number";
    const n = Number(value);
    const min = spec.min ?? 0;
    const max = spec.max ?? Number.MAX_SAFE_INTEGER;
    return n >= min && n <= max ? null : `must be between ${String(min)} and ${String(max)}`;
}

function checkFontFamily(value: string): string | null {
    const families = value.split(",").map((f) => f.trim());
    if (families.length > LIMITS.maxFontFamilies) return `at most ${String(LIMITS.maxFontFamilies)} fonts`;
    for (const family of families) {
        const ok = FONT_QUOTED.test(family) || FONT_BARE.test(family) || GENERIC_FONTS.has(family);
        if (!ok) return `\`${family.slice(0, 30)}\` is not a plain font name`;
    }
    return null;
}

/** Returns an error message if `value` is not valid for `spec`, otherwise `null`. */
export function checkTokenValue(spec: TokenSpec, value: string): string | null {
    if (value.length === 0 || value.length > LIMITS.maxValueLength) return "empty or too long";
    switch (spec.kind) {
        case "color":
            return isValidColor(value) ? null : "expected a color: #rgb, #rrggbb, #rrggbbaa, rgb()/rgba()/hsl()/hsla() or transparent";
        case "rgb-triplet":
            return checkTriplet(value);
        case "length":
            return checkLength(value, spec);
        case "number":
            return checkNumber(value, spec);
        case "font-family":
            return checkFontFamily(value);
    }
}

// --- parsing --------------------------------------------------------------------------------

function isRecord(value: unknown): value is Record<string, unknown> {
    return typeof value === "object" && value !== null && !Array.isArray(value);
}

function cleanString(value: unknown, path: string, issues: ValidationIssue[], required: boolean): string | undefined {
    if (value === undefined) {
        if (required) issues.push({ severity: "error", path, message: "is required" });
        return undefined;
    }
    if (typeof value !== "string") {
        issues.push({ severity: "error", path, message: "must be a string" });
        return undefined;
    }
    // Strip control characters (including newlines) before the text reaches the UI.
    // eslint-disable-next-line no-control-regex
    const text = value.replace(/[\u0000-\u001f\u007f]/g, " ").trim();
    if (text.length === 0 && required) {
        issues.push({ severity: "error", path, message: "must not be empty" });
        return undefined;
    }
    if (text.length > LIMITS.maxMetadataLength) {
        issues.push({ severity: "error", path, message: `must be at most ${String(LIMITS.maxMetadataLength)} characters` });
        return undefined;
    }
    return text;
}

function parseMetadata(raw: unknown, issues: ValidationIssue[]): ThemeMetadata | undefined {
    if (!isRecord(raw)) {
        issues.push({ severity: "error", path: "metadata", message: "is required and must be an object" });
        return undefined;
    }
    const name = cleanString(raw.name, "metadata.name", issues, true);
    const author = cleanString(raw.author, "metadata.author", issues, false);
    const version = cleanString(raw.version, "metadata.version", issues, false);
    const description = cleanString(raw.description, "metadata.description", issues, false);
    if (name === undefined) return undefined;
    const metadata: ThemeMetadata = { name };
    if (author !== undefined) metadata.author = author;
    if (version !== undefined) metadata.version = version;
    if (description !== undefined) metadata.description = description;
    return metadata;
}

function parseTokens(raw: unknown, issues: ValidationIssue[]): Record<string, string> {
    const tokens: Record<string, string> = {};
    if (!isRecord(raw)) {
        issues.push({ severity: "error", path: "tokens", message: "is required and must be an object" });
        return tokens;
    }
    const entries = Object.entries(raw);
    if (entries.length > LIMITS.maxTokens) {
        issues.push({ severity: "error", path: "tokens", message: `has more than ${String(LIMITS.maxTokens)} entries` });
        return tokens;
    }
    for (const [key, value] of entries) {
        const path = `tokens.${key}`;
        const cssName = `--${key}`;
        const spec = getTokenSpec(cssName);
        if (spec === undefined) {
            issues.push({ severity: "warning", path, message: "is not a themable token and was ignored" });
            continue;
        }
        if (typeof value !== "string") {
            issues.push({ severity: "error", path, message: "must be a string" });
            continue;
        }
        const problem = checkTokenValue(spec, value.trim());
        if (problem !== null) {
            issues.push({ severity: "error", path, message: problem });
            continue;
        }
        tokens[cssName] = value.trim();
    }
    return tokens;
}

function parseJson(text: string): { value: Record<string, unknown> } | { errors: ValidationIssue[] } {
    if (text.length > LIMITS.maxFileLength) {
        return {
            errors: [{ severity: "error", path: "", message: `file is larger than ${String(LIMITS.maxFileLength / 1024)} KB` }],
        };
    }
    let raw: unknown;
    try {
        raw = JSON.parse(text);
    } catch (e) {
        const detail = e instanceof Error ? e.message : "invalid JSON";
        return { errors: [{ severity: "error", path: "", message: `not valid JSON (${detail})` }] };
    }
    if (!isRecord(raw)) {
        return { errors: [{ severity: "error", path: "", message: "the top level must be an object" }] };
    }
    return { value: raw };
}

function checkSchema(schema: unknown, issues: ValidationIssue[]): void {
    if (schema === THEME_SCHEMA_VERSION) return;
    const found = typeof schema === "number" ? String(schema) : "missing";
    issues.push({
        severity: "error",
        path: "schema",
        message: `unsupported schema version (${found}); this version of the app reads schema ${String(THEME_SCHEMA_VERSION)}`,
    });
}

function parseBase(raw: unknown, issues: ValidationIssue[]): ThemeBase {
    if (raw === undefined) {
        issues.push({ severity: "warning", path: "base", message: 'missing, assuming "dark"' });
        return "dark";
    }
    if (typeof raw === "string" && (THEME_BASES as readonly string[]).includes(raw)) return raw as ThemeBase;
    issues.push({ severity: "error", path: "base", message: 'must be "dark" or "light"' });
    return "dark";
}

/** Parses and validates the text of a theme file. Never throws. */
export function parseTheme(text: string): ParseResult {
    const parsed = parseJson(text);
    if ("errors" in parsed) return { ok: false, errors: parsed.errors };
    const raw = parsed.value;

    const issues: ValidationIssue[] = [];
    checkSchema(raw.schema, issues);
    const metadata = parseMetadata(raw.metadata, issues);
    const base = parseBase(raw.base, issues);
    const tokens = parseTokens(raw.tokens, issues);

    const errors = issues.filter((i) => i.severity === "error");
    if (errors.length > 0 || metadata === undefined) {
        return { ok: false, errors };
    }
    return {
        ok: true,
        theme: { schema: THEME_SCHEMA_VERSION, metadata, base, tokens },
        warnings: issues.filter((i) => i.severity === "warning"),
    };
}
