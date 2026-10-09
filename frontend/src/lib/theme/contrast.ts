/**
 * WCAG contrast helpers, used to *warn* theme authors about unreadable color choices
 * (never to reject a theme: taste is the author's call).
 */

export interface Rgba {
    r: number;
    g: number;
    b: number;
    /** 0 (transparent) to 1 (opaque). */
    a: number;
}

const COMPUTED_RGB = /^rgba?\(\s*([\d.]+)[\s,]+([\d.]+)[\s,]+([\d.]+)(?:\s*[,/]\s*([\d.]+%?))?\s*\)$/i;
const SRGB_FUNCTION = /^color\(srgb\s+([\d.]+)\s+([\d.]+)\s+([\d.]+)(?:\s*\/\s*([\d.]+%?))?\s*\)$/i;

function parseAlpha(raw: string | undefined): number {
    if (raw === undefined) return 1;
    return raw.endsWith("%") ? Number.parseFloat(raw) / 100 : Number(raw);
}

/**
 * Parses a color as reported by `getComputedStyle` (`rgb()`, `rgba()` or `color(srgb ...)`).
 * Returns `null` for anything else.
 */
export function parseComputedColor(value: string): Rgba | null {
    const text = value.trim();
    const rgb = COMPUTED_RGB.exec(text);
    if (rgb !== null) {
        return { r: Number(rgb[1]), g: Number(rgb[2]), b: Number(rgb[3]), a: parseAlpha(rgb[4]) };
    }
    const srgb = SRGB_FUNCTION.exec(text);
    if (srgb !== null) {
        return {
            r: Math.round(Number(srgb[1]) * 255),
            g: Math.round(Number(srgb[2]) * 255),
            b: Math.round(Number(srgb[3]) * 255),
            a: parseAlpha(srgb[4]),
        };
    }
    return null;
}

/** Composites `top` over an opaque `bottom`. */
export function compositeOver(top: Rgba, bottom: Rgba): Rgba {
    const a = top.a;
    return {
        r: Math.round(top.r * a + bottom.r * (1 - a)),
        g: Math.round(top.g * a + bottom.g * (1 - a)),
        b: Math.round(top.b * a + bottom.b * (1 - a)),
        a: 1,
    };
}

function channel(value: number): number {
    const v = value / 255;
    return v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4;
}

export function relativeLuminance(c: Rgba): number {
    return 0.2126 * channel(c.r) + 0.7152 * channel(c.g) + 0.0722 * channel(c.b);
}

/** WCAG 2.x contrast ratio between two opaque colors, from 1 to 21. */
export function contrastRatio(foreground: Rgba, background: Rgba): number {
    const l1 = relativeLuminance(foreground);
    const l2 = relativeLuminance(background);
    const [lighter, darker] = l1 >= l2 ? [l1, l2] : [l2, l1];
    return (lighter + 0.05) / (darker + 0.05);
}

/** WCAG AA threshold for normal-size text. */
export const AA_NORMAL_TEXT = 4.5;
