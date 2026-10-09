/**
 * Applying a theme to the document.
 *
 * Built-in themes are plain CSS classes on `<html>` (`theme-dark`, `theme-light`, ...) defined
 * in `styles/themes.css`, so the first paint needs no JavaScript. A user theme is rendered as
 * the class of its `base` theme plus a handful of validated inline custom properties.
 *
 * Properties are set with the CSSOM (`style.setProperty`), never by writing CSS text, which is
 * also what keeps this compatible with the app's Content Security Policy.
 */
import type { ThemeBase } from "./tokens";
import type { ThemeDefinition } from "./validate";

export interface BuiltinTheme {
    /** CSS class suffix: the class is `theme-<id>`. */
    readonly id: string;
    /** Value stored in the configuration (`MappingConfig.theme`). */
    readonly configValue: string;
    readonly label: string;
    readonly scheme: ThemeBase;
}

export const BUILTIN_THEMES: readonly BuiltinTheme[] = [
    { id: "dark", configValue: "Dark", label: "Dark", scheme: "dark" },
    { id: "light", configValue: "Light", label: "Light", scheme: "light" },
    { id: "catppuccinlatte", configValue: "CatppuccinLatte", label: "Catppuccin Latte", scheme: "light" },
    { id: "catppuccinfrappe", configValue: "CatppuccinFrappe", label: "Catppuccin Frappe", scheme: "dark" },
    { id: "catppuccinmacchiato", configValue: "CatppuccinMacchiato", label: "Catppuccin Macchiato", scheme: "dark" },
    { id: "catppuccinmocha", configValue: "CatppuccinMocha", label: "Catppuccin Mocha", scheme: "dark" },
];

/** Configuration value meaning "follow the operating system". */
export const SYSTEM_THEME = "System";
/** Prefix of a user theme in the configuration: `custom:<file id>`. */
export const CUSTOM_PREFIX = "custom:";
/** Theme used when the selected one cannot be found or is invalid. */
export const FALLBACK_THEME_ID = "dark";

/** What has to be written on `<html>` for a theme. */
export interface ThemeRender {
    /** Class on `<html>`, e.g. `theme-light`. */
    themeClass: string;
    scheme: ThemeBase;
    /** Inline custom properties (user themes only). */
    tokens: Record<string, string>;
}

export interface ResolvedTheme {
    render: ThemeRender;
    /** Human-readable name, for the UI. */
    label: string;
    /** Set when the selection could not be honoured and the fallback was used instead. */
    fallbackReason?: string;
}

export function builtinById(id: string): BuiltinTheme | undefined {
    return BUILTIN_THEMES.find((t) => t.id === id);
}

function builtinByConfigValue(value: string): BuiltinTheme | undefined {
    const lowered = value.toLowerCase();
    return BUILTIN_THEMES.find((t) => t.configValue.toLowerCase() === lowered || t.id === lowered);
}

function renderBuiltin(theme: BuiltinTheme): ThemeRender {
    return { themeClass: `theme-${theme.id}`, scheme: theme.scheme, tokens: {} };
}

/**
 * Turns a configuration value into what must be applied.
 *
 * @param selection - `System`, a built-in `configValue`, or `custom:<id>`.
 * @param prefersDark - the OS preference, used for `System`.
 * @param userThemes - validated user themes by id.
 */
export function resolveTheme(
    selection: string,
    prefersDark: boolean,
    userThemes: ReadonlyMap<string, ThemeDefinition>,
): ResolvedTheme {
    if (selection.toLowerCase() === SYSTEM_THEME.toLowerCase()) {
        const theme = builtinById(prefersDark ? "dark" : "light");
        if (theme !== undefined) return { render: renderBuiltin(theme), label: "System" };
    }

    const builtin = builtinByConfigValue(selection);
    if (builtin !== undefined) return { render: renderBuiltin(builtin), label: builtin.label };

    if (selection.startsWith(CUSTOM_PREFIX)) {
        const id = selection.slice(CUSTOM_PREFIX.length);
        const definition = userThemes.get(id);
        if (definition !== undefined) {
            return {
                render: { themeClass: `theme-${definition.base}`, scheme: definition.base, tokens: definition.tokens },
                label: definition.metadata.name,
            };
        }
        return fallback(`The theme "${id}" was not found or is invalid.`);
    }

    return fallback(`Unknown theme "${selection}".`);
}

function fallback(reason: string): ResolvedTheme {
    const theme = builtinById(FALLBACK_THEME_ID);
    if (theme === undefined) throw new Error("The fallback theme is missing from BUILTIN_THEMES");
    return { render: renderBuiltin(theme), label: theme.label, fallbackReason: reason };
}

/**
 * Names of the custom properties currently set by a user theme, kept on the element itself so
 * the first-paint script (`public/theme-boot.js`) and this module agree on what to clean up.
 */
const APPLIED_ATTRIBUTE = "data-theme-tokens";

function previouslyApplied(root: HTMLElement): string[] {
    const value = root.getAttribute(APPLIED_ATTRIBUTE);
    return value === null || value === "" ? [] : value.split(" ");
}

/** Writes a theme on `root`, replacing whatever theme was applied before. */
export function applyThemeRender(root: HTMLElement, render: ThemeRender): void {
    for (const cls of [...root.classList]) {
        if (cls.startsWith("theme-")) root.classList.remove(cls);
    }
    root.classList.add(render.themeClass);

    for (const name of previouslyApplied(root)) root.style.removeProperty(name);
    const applied: string[] = [];
    for (const [name, value] of Object.entries(render.tokens)) {
        root.style.setProperty(name, value);
        applied.push(name);
    }
    if (applied.length > 0) root.setAttribute(APPLIED_ATTRIBUTE, applied.join(" "));
    else root.removeAttribute(APPLIED_ATTRIBUTE);

    // Built-in themes declare `color-scheme` in CSS; a user theme gets it from its base.
    if (applied.length > 0) root.style.setProperty("color-scheme", render.scheme);
    else root.style.removeProperty("color-scheme");
}

// --- first-paint persistence ------------------------------------------------------------------

/** localStorage key read by `public/theme-boot.js` before the first paint. */
export const THEME_STORAGE_KEY = "ntd.theme.v1";

/**
 * Remembers the resolved theme so the next launch can paint it immediately, before the
 * configuration has been loaded. Best effort: storage may be unavailable.
 */
export function rememberTheme(render: ThemeRender): void {
    try {
        localStorage.setItem(THEME_STORAGE_KEY, JSON.stringify(render));
    } catch {
        /* private mode, storage disabled... the theme simply applies a moment later */
    }
}
