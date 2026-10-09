/**
 * The theming contract: every design token a user theme is allowed to override.
 *
 * This list is the single source of truth. `styles/tokens.css` declares the defaults, the
 * validator (`validate.ts`) only accepts the names and value grammars listed here, and the
 * documentation / JSON Schema are generated from it. `design-tokens.test.ts` fails if this
 * list and `tokens.css` drift apart.
 *
 * Anything not listed (spacing scale, font sizes, z-index, motion...) is deliberately not
 * themable: changing it can break layout, so themes are limited to look, not structure.
 */

/** How a token's value is written in a theme file. */
export type TokenKind =
    /** `#rgb`, `#rrggbb`, `#rrggbbaa`, `rgb()/rgba()/hsl()/hsla()` or `transparent`. */
    | "color"
    /** Space-separated 0-255 channels, e.g. `255 255 255` (used with `rgb(... / alpha)`). */
    | "rgb-triplet"
    /** A non-negative CSS length in px/rem/em, bounded by `max`. */
    | "length"
    /** A comma-separated font stack. */
    | "font-family"
    /** A plain number between `min` and `max`. */
    | "number";

export type TokenGroup =
    | "surface"
    | "text"
    | "accent"
    | "border"
    | "status"
    | "overlay"
    | "input"
    | "visualization"
    | "console"
    | "titlebar"
    | "shape"
    | "typography";

export interface TokenSpec {
    /** CSS custom property name, including the leading `--`. */
    readonly name: string;
    readonly kind: TokenKind;
    readonly group: TokenGroup;
    readonly description: string;
    /** Upper bound for `length` (in px) and `number` tokens. */
    readonly max?: number;
    /** Lower bound for `number` tokens. */
    readonly min?: number;
}

const color = (name: string, group: TokenGroup, description: string): TokenSpec => ({
    name,
    kind: "color",
    group,
    description,
});

const radius = (name: string, description: string): TokenSpec => ({
    name,
    kind: "length",
    group: "shape",
    description,
    max: 32,
});

export const THEMABLE_TOKENS: readonly TokenSpec[] = [
    // Surfaces
    color("--bg-app", "surface", "Window background."),
    color("--bg-panel", "surface", "Cards, groups, menus and panels."),
    color("--bg-panel-hover", "surface", "Panels and menu items under the pointer."),
    color("--bg-input", "surface", "Background of inputs and dropdown triggers."),
    color("--bg-tab-active", "surface", "Background of the active tab."),

    // Text
    color("--text-main", "text", "Normal text."),
    color("--text-muted", "text", "Secondary text, hints and icons."),
    color("--text-active", "text", "Titles and hovered text."),
    color("--text-strong", "text", "Names and headings one step above normal text."),
    color("--text-emphasis", "text", "The strongest text (statistics, highlighted names)."),
    color("--text-on-accent", "text", "Text drawn on an accent-colored background."),
    color("--text-number", "text", "Numeric values in inputs."),

    // Accent
    color("--accent", "accent", "Main theme color: selections, checkboxes, active widgets."),
    color("--accent-hover", "accent", "Accent while hovered."),

    // Borders
    color("--border", "border", "Borders between cards and panels."),
    color("--border-hover", "border", "Borders of interactive cards while hovered."),

    // Status
    color("--info", "status", "Informational status."),
    color("--warning", "status", "Warning status."),
    color("--error", "status", "Error status (log levels, validation)."),
    color("--danger", "status", "Destructive actions."),
    color("--danger-hover", "status", "Destructive actions while hovered."),
    color("--success", "status", "Success / running status."),
    color("--debug", "status", "Debug-level log lines."),

    // Overlays
    {
        name: "--overlay-rgb",
        kind: "rgb-triplet",
        group: "overlay",
        description: "Channels of the color used for translucent highlights (white on dark themes, black on light themes).",
    },
    {
        name: "--shade-rgb",
        kind: "rgb-triplet",
        group: "overlay",
        description: "Channels of the color used for translucent shadows and darkening.",
    },

    // Inputs
    color("--input-bg", "input", "Input field background."),
    color("--input-bg-hover", "input", "Input field background while hovered."),
    color("--input-bg-focus", "input", "Input field background while focused."),
    color("--input-border", "input", "Input field border."),
    color("--input-border-hover", "input", "Input field border while hovered."),

    // Tablet / display visualisation
    color("--area-display", "visualization", "Display area in the output preview."),
    color("--area-tablet", "visualization", "Tablet area in the output preview."),
    color("--area-border-main", "visualization", "Outline of the tablet area."),
    color("--area-border-pink", "visualization", "Outline of the osu! playfield."),
    color("--area-playfield", "visualization", "Fill of the osu! playfield rectangle."),
    color("--area-contrast", "visualization", "Text and handles drawn over the preview areas."),

    // Console
    color("--console-text", "console", "Log message text."),

    // Title bar
    color("--titlebar-close-bg", "titlebar", "Close button background while hovered."),
    color("--titlebar-close-fg", "titlebar", "Close button icon while hovered."),

    // Shape
    radius("--radius-sm", "Smallest corner radius."),
    radius("--radius-md", "Default corner radius (buttons, inputs)."),
    radius("--radius-lg", "Large corner radius (panels, menus)."),
    radius("--radius-xl", "Extra large corner radius (cards)."),
    {
        name: "--disabled-opacity",
        kind: "number",
        group: "shape",
        description: "Opacity of disabled controls (0 to 1).",
        min: 0,
        max: 1,
    },

    // Typography
    {
        name: "--font-sans",
        kind: "font-family",
        group: "typography",
        description: "Interface font stack. Only installed fonts can be used.",
    },
    {
        name: "--font-mono",
        kind: "font-family",
        group: "typography",
        description: "Monospace font stack (console, hashes).",
    },
];

const BY_NAME: ReadonlyMap<string, TokenSpec> = new Map(THEMABLE_TOKENS.map((t) => [t.name, t]));

export function getTokenSpec(name: string): TokenSpec | undefined {
    return BY_NAME.get(name);
}

/** Token names in the form used inside a theme file (without the leading `--`). */
export function toThemeKey(cssName: string): string {
    return cssName.replace(/^--/, "");
}

/** The bases a user theme can be built on. Missing tokens fall back to this base. */
export const THEME_BASES = ["dark", "light"] as const;
export type ThemeBase = (typeof THEME_BASES)[number];

/** Current theme file format version. */
export const THEME_SCHEMA_VERSION = 1;
