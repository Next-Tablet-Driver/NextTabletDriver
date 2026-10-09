import { SvelteMap } from "svelte/reactivity";
import {
    AA_NORMAL_TEXT,
    compositeOver,
    contrastRatio,
    parseComputedColor,
    type Rgba,
} from "./contrast";
import {
    applyThemeRender,
    rememberTheme,
    CUSTOM_PREFIX,
    resolveTheme,
    type ResolvedTheme,
} from "./apply";
import { parseTheme, type ThemeDefinition, type ValidationIssue } from "./validate";

/** A user theme file as handed over by the backend. */
export interface UserThemeSource {
    /** Stable id derived from the file name (what `custom:<id>` refers to). */
    id: string;
    fileName: string;
    content: string;
}

export interface UserThemeEntry {
    id: string;
    fileName: string;
    /** Present when the file is valid. */
    definition?: ThemeDefinition;
    /** Why the file cannot be used (empty when valid). */
    errors: ValidationIssue[];
    warnings: ValidationIssue[];
}

/** Reads a CSS custom property as a color, or `null` if it cannot be resolved. */
export type ColorResolver = (cssVariable: string) => Rgba | null;

/** Resolves a token through a throw-away element so `var()` chains and color-mix() are evaluated. */
export function domColorResolver(root: HTMLElement): ColorResolver {
    return (cssVariable) => {
        const probe = document.createElement("span");
        probe.style.cssText = `position:absolute;visibility:hidden;color:var(${cssVariable})`;
        root.append(probe);
        const value = getComputedStyle(probe).color;
        probe.remove();
        return parseComputedColor(value);
    };
}

interface ContrastPair {
    foreground: string;
    background: string;
    minimum: number;
    description: string;
}

const CONTRAST_PAIRS: readonly ContrastPair[] = [
    { foreground: "--text-main", background: "--bg-app", minimum: AA_NORMAL_TEXT, description: "Text on the window background" },
    { foreground: "--text-main", background: "--bg-panel", minimum: AA_NORMAL_TEXT, description: "Text on panels" },
    { foreground: "--text-muted", background: "--bg-panel", minimum: 3, description: "Secondary text on panels" },
    { foreground: "--text-on-accent", background: "--accent", minimum: AA_NORMAL_TEXT, description: "Text on accent-colored buttons" },
];

/** Lists the readability problems of the currently applied theme (empty when fine or unknown). */
export function contrastWarnings(resolve: ColorResolver): string[] {
    const warnings: string[] = [];
    for (const pair of CONTRAST_PAIRS) {
        const fg = resolve(pair.foreground);
        const bg = resolve(pair.background);
        if (fg === null || bg === null) continue;
        const opaqueBg = compositeOver(bg, { r: 0, g: 0, b: 0, a: 1 });
        const ratio = contrastRatio(compositeOver(fg, opaqueBg), opaqueBg);
        if (ratio < pair.minimum) {
            warnings.push(
                `${pair.description} has a contrast of ${ratio.toFixed(1)}:1 (at least ${String(pair.minimum)}:1 is recommended).`,
            );
        }
    }
    return warnings;
}

/**
 * The one place that decides which theme is on screen.
 *
 * Holds the user's selection (the `theme` string of the configuration), the available user
 * themes, and re-applies the theme when either changes or when the OS color scheme changes
 * (for `System`).
 */
export class ThemeStore {
    /** The configuration value: `System`, a built-in name, or `custom:<id>`. */
    selection = $state("Dark");
    userThemes = $state<UserThemeEntry[]>([]);
    /** False until the theme files have been read once (see `apply`). */
    userThemesLoaded = $state(false);
    resolved = $state<ResolvedTheme | null>(null);
    warnings = $state<string[]>([]);

    readonly #root: HTMLElement;
    readonly #resolveColor: ColorResolver;
    readonly #media: MediaQueryList | null;
    readonly #onSchemeChange = (): void => {
        if (this.selection.toLowerCase() === "system") this.apply();
    };

    constructor(
        root: HTMLElement = document.documentElement,
        media: MediaQueryList | null = typeof window.matchMedia === "function"
            ? window.matchMedia("(prefers-color-scheme: dark)")
            : null,
        resolveColor: ColorResolver = domColorResolver(root),
    ) {
        this.#root = root;
        this.#media = media;
        this.#resolveColor = resolveColor;
    }

    /** Starts following OS color-scheme changes. Call `stop()` to release the listener. */
    start(): void {
        this.#media?.addEventListener("change", this.#onSchemeChange);
    }

    stop(): void {
        this.#media?.removeEventListener("change", this.#onSchemeChange);
    }

    /** Changes the selected theme and applies it. */
    select(value: string): void {
        this.selection = value;
        this.apply();
    }

    /** Replaces the list of user themes (parsing and validating every file) and re-applies. */
    setUserThemes(sources: readonly UserThemeSource[]): void {
        this.userThemesLoaded = true;
        this.userThemes = sources.map((source): UserThemeEntry => {
            const result = parseTheme(source.content);
            return result.ok
                ? { id: source.id, fileName: source.fileName, definition: result.theme, errors: [], warnings: result.warnings }
                : { id: source.id, fileName: source.fileName, errors: result.errors, warnings: [] };
        });
        this.apply();
    }

    get validUserThemes(): ReadonlyMap<string, ThemeDefinition> {
        const map = new SvelteMap<string, ThemeDefinition>();
        for (const entry of this.userThemes) {
            if (entry.definition !== undefined) map.set(entry.id, entry.definition);
        }
        return map;
    }

    /** Resolves the selection and writes it on the document. */
    apply(): void {
        // A custom theme cannot be resolved before its file has been read. Until then keep what
        // the first-paint script put on screen instead of flashing the fallback.
        if (!this.userThemesLoaded && this.selection.startsWith(CUSTOM_PREFIX)) return;

        const prefersDark = this.#media?.matches ?? true;
        const resolved = resolveTheme(this.selection, prefersDark, this.validUserThemes);
        applyThemeRender(this.#root, resolved.render);
        rememberTheme(resolved.render);
        this.resolved = resolved;

        const isCustom = Object.keys(resolved.render.tokens).length > 0;
        this.warnings = isCustom ? contrastWarnings(this.#resolveColor) : [];
    }
}
