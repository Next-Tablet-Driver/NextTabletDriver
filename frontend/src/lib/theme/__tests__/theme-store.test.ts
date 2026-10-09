import { describe, it, expect, beforeEach, vi } from "vitest";
import { ThemeStore, contrastWarnings, type ColorResolver } from "../theme-store.svelte";

function fakeMedia(initialDark: boolean): { media: MediaQueryList; setDark: (dark: boolean) => void } {
    const listeners = new Set<() => void>();
    const state = { matches: initialDark };
    const media = {
        get matches() {
            return state.matches;
        },
        addEventListener: (_: string, fn: () => void) => listeners.add(fn),
        removeEventListener: (_: string, fn: () => void) => listeners.delete(fn),
    } as unknown as MediaQueryList;
    return {
        media,
        setDark: (dark) => {
            state.matches = dark;
            for (const fn of listeners) fn();
        },
    };
}

const noColors: ColorResolver = () => null;

function themeFile(name: string, tokens: Record<string, string> = { accent: "#ff007f" }, base = "dark"): string {
    return JSON.stringify({ schema: 1, metadata: { name }, base, tokens });
}

describe("ThemeStore", () => {
    let root: HTMLElement;
    beforeEach(() => {
        root = document.createElement("html");
        localStorage.clear();
    });

    it("applies the selected built-in theme", () => {
        const store = new ThemeStore(root, fakeMedia(true).media, noColors);
        store.select("Light");
        expect(root.classList.contains("theme-light")).toBe(true);
        expect(store.resolved?.label).toBe("Light");
        store.select("CatppuccinMocha");
        expect(root.classList.contains("theme-light")).toBe(false);
        expect(root.classList.contains("theme-catppuccinmocha")).toBe(true);
    });

    it("follows OS changes live while System is selected, and only then", () => {
        const { media, setDark } = fakeMedia(true);
        const store = new ThemeStore(root, media, noColors);
        store.start();
        store.select("System");
        expect(root.classList.contains("theme-dark")).toBe(true);

        setDark(false);
        expect(root.classList.contains("theme-light")).toBe(true);

        store.select("Dark");
        setDark(false);
        expect(root.classList.contains("theme-dark")).toBe(true);

        store.stop();
        store.select("System");
        setDark(true);
        expect(root.classList.contains("theme-light")).toBe(true); // no longer listening
    });

    it("loads, validates and applies user themes", () => {
        const store = new ThemeStore(root, fakeMedia(true).media, noColors);
        store.setUserThemes([
            { id: "neon", fileName: "neon.json", content: themeFile("Neon", { accent: "#ff007f" }) },
            { id: "broken", fileName: "broken.json", content: "{ not json" },
        ]);
        expect(store.userThemes.map((t) => [t.id, t.definition !== undefined])).toEqual([
            ["neon", true],
            ["broken", false],
        ]);
        expect(store.userThemes[1]?.errors[0]?.message).toMatch(/not valid JSON/);

        store.select("custom:neon");
        expect(root.style.getPropertyValue("--accent")).toBe("#ff007f");
        expect(store.resolved?.label).toBe("Neon");
    });

    it("falls back to dark when the selected user theme is invalid or gone", () => {
        const store = new ThemeStore(root, fakeMedia(false).media, noColors);
        store.setUserThemes([{ id: "broken", fileName: "broken.json", content: "[]" }]);
        store.select("custom:broken");
        expect(root.classList.contains("theme-dark")).toBe(true);
        expect(store.resolved?.fallbackReason).toBeDefined();

        store.select("custom:missing");
        expect(store.resolved?.fallbackReason).toMatch(/missing/);
    });

    it("keeps the first-paint theme until the user theme files have been read", () => {
        root.classList.add("theme-light"); // what theme-boot.js painted
        const store = new ThemeStore(root, fakeMedia(true).media, noColors);
        store.select("custom:neon");
        expect(root.classList.contains("theme-light")).toBe(true);
        expect(store.resolved).toBeNull();

        store.setUserThemes([{ id: "neon", fileName: "neon.json", content: themeFile("Neon", { accent: "#ff007f" }, "light") }]);
        expect(root.style.getPropertyValue("--accent")).toBe("#ff007f");
        expect(store.resolved?.fallbackReason).toBeUndefined();
    });

    it("re-applies when the user theme list changes (e.g. after a reload of the folder)", () => {
        const store = new ThemeStore(root, fakeMedia(true).media, noColors);
        store.setUserThemes([]);
        store.select("custom:neon");
        expect(store.resolved?.fallbackReason).toBeDefined();
        store.setUserThemes([{ id: "neon", fileName: "neon.json", content: themeFile("Neon") }]);
        expect(store.resolved?.fallbackReason).toBeUndefined();
    });

    it("reports contrast problems of a user theme but not of built-in themes", () => {
        const resolver = vi.fn<ColorResolver>((name) =>
            name === "--text-main" ? { r: 120, g: 120, b: 120, a: 1 } : { r: 128, g: 128, b: 128, a: 1 },
        );
        const store = new ThemeStore(root, fakeMedia(true).media, resolver);
        store.select("Dark");
        expect(store.warnings).toEqual([]);

        store.setUserThemes([{ id: "dim", fileName: "dim.json", content: themeFile("Dim") }]);
        store.select("custom:dim");
        expect(store.warnings.length).toBeGreaterThan(0);
        expect(store.warnings[0]).toMatch(/contrast of 1\.\d:1/);
    });
});

describe("contrastWarnings", () => {
    it("is silent for readable colors and for colors that cannot be resolved", () => {
        const readable: ColorResolver = (name) =>
            name === "--text-main" || name === "--text-muted" || name === "--text-on-accent"
                ? { r: 255, g: 255, b: 255, a: 1 }
                : { r: 20, g: 20, b: 30, a: 1 };
        expect(contrastWarnings(readable)).toEqual([]);
        expect(contrastWarnings(noColors)).toEqual([]);
    });
});
