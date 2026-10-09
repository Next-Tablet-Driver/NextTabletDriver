/**
 * JSON Schema (draft-07) for theme files, generated from the token contract.
 *
 * It gives theme authors autocompletion and inline errors in any editor that understands
 * `"$schema"`. It is a convenience: the app's own validator (`validate.ts`) is authoritative and
 * slightly stricter in places a schema cannot express.
 */
import { THEMABLE_TOKENS, THEME_BASES, THEME_SCHEMA_VERSION, toThemeKey, type TokenSpec } from "./tokens";
import { LIMITS } from "./validate";

export const SCHEMA_URL =
    "https://raw.githubusercontent.com/Next-Tablet-Driver/NextTabletDriver/master/docs/themes/theme.schema.json";

const HEX = "#(?:[0-9a-fA-F]{3,4}|[0-9a-fA-F]{6}|[0-9a-fA-F]{8})";
const FUNCTION = "(?:rgb|rgba|hsl|hsla)\\([0-9., %/degDEG]+\\)";

const PATTERNS: Record<TokenSpec["kind"], string> = {
    color: `^(?:transparent|${HEX}|${FUNCTION})$`,
    "rgb-triplet": "^[0-9]{1,3} [0-9]{1,3} [0-9]{1,3}$",
    length: "^[0-9]+(?:\\.[0-9]+)?(?:px|rem|em)$",
    number: "^[0-9]+(?:\\.[0-9]+)?$",
    "font-family": "^[A-Za-z0-9 ,'\"._-]+$",
};

const KIND_HINT: Record<TokenSpec["kind"], string> = {
    color: "A color: #rgb, #rrggbb, #rrggbbaa, rgb()/rgba()/hsl()/hsla() or transparent.",
    "rgb-triplet": "Three channels from 0 to 255 separated by spaces, e.g. `255 255 255`.",
    length: "A length in px, rem or em.",
    number: "A number.",
    "font-family": "A comma-separated list of installed font names.",
};

function tokenSchema(spec: TokenSpec): Record<string, unknown> {
    const bounds =
        spec.kind === "length" && spec.max !== undefined
            ? ` At most ${String(spec.max)}px.`
            : spec.kind === "number"
              ? ` Between ${String(spec.min ?? 0)} and ${String(spec.max ?? 1)}.`
              : "";
    return {
        type: "string",
        description: `${spec.description} ${KIND_HINT[spec.kind]}${bounds}`,
        pattern: PATTERNS[spec.kind],
        maxLength: LIMITS.maxValueLength,
    };
}

export function buildThemeJsonSchema(): Record<string, unknown> {
    const tokens: Record<string, unknown> = {};
    for (const spec of THEMABLE_TOKENS) tokens[toThemeKey(spec.name)] = tokenSchema(spec);

    const text = (description: string): Record<string, unknown> => ({
        type: "string",
        description,
        maxLength: LIMITS.maxMetadataLength,
    });

    return {
        $schema: "http://json-schema.org/draft-07/schema#",
        $id: SCHEMA_URL,
        title: "NextTabletDriver theme",
        description: "A user theme: overrides of the design tokens listed under `tokens`.",
        type: "object",
        required: ["schema", "metadata", "tokens"],
        properties: {
            $schema: { type: "string", description: "Optional: this schema's URL, for editor support." },
            schema: { const: THEME_SCHEMA_VERSION, description: "Theme file format version." },
            metadata: {
                type: "object",
                required: ["name"],
                additionalProperties: false,
                properties: {
                    name: { ...text("Name shown in Settings > Themes."), minLength: 1 },
                    author: text("Your name or alias."),
                    version: text("Theme version, e.g. `1.0`."),
                    description: text("One line about the theme."),
                },
            },
            base: {
                enum: [...THEME_BASES],
                default: "dark",
                description: "Built-in theme that every token you do not set falls back to.",
            },
            tokens: {
                type: "object",
                description: "Token overrides. Names are the CSS variable names without the leading `--`.",
                additionalProperties: false,
                maxProperties: LIMITS.maxTokens,
                properties: tokens,
            },
        },
    };
}

/** The schema as committed in `docs/themes/theme.schema.json`. */
export function themeJsonSchemaText(): string {
    return `${JSON.stringify(buildThemeJsonSchema(), null, 2)}\n`;
}
