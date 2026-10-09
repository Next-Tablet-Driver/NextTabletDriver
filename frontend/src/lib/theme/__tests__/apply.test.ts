import { describe, it, expect, beforeEach } from "vitest";
import {
    BUILTIN_THEMES,
    THEME_STORAGE_KEY,
    applyThemeRender,
    rememberTheme,
    resolveTheme,
} from "../apply";
import type { ThemeDefinition } from "../validate";

const custom: ThemeDefinition = {
    schema: 1,
    metadata: { name: "Neon" },
    base: "light",
    tokens: { "--accent": "#ff007f", "--bg-app": "#fafafa" },
};
const userThemes = new Map([["neon", custom]]);

describe("resolveTheme", () => {
    it("maps every built-in config value (case-insensitively) to its class", () => {
        for (const theme of BUILTIN_THEMES) {
            const resolved = resolveTheme(theme.configValue, true, userThemes);
            expect(resolved.render.themeClass).toBe(`theme-${theme.id}`);
            expect(resolved.render.scheme).toBe(theme.scheme);
            expect(resolved.fallbackReason).toBeUndefined();
        }
        expect(resolveTheme("catppuccinmocha", true, userThemes).render.themeClass).toBe("theme-catppuccinmocha");
    });

    it("follows the OS preference for System", () => {
        expect(resolveTheme("System", true, userThemes).render.themeClass).toBe("theme-dark");
        expect(resolveTheme("System", false, userThemes).render.themeClass).toBe("theme-light");
        expect(resolveTheme("system", false, userThemes).render.themeClass).toBe("theme-light");
    });

    it("renders a user theme as its base class plus its tokens", () => {
        const resolved = resolveTheme("custom:neon", true, userThemes);
        expect(resolved.render).toEqual({ themeClass: "theme-light", scheme: "light", tokens: custom.tokens });
        expect(resolved.label).toBe("Neon");
    });

    it("falls back to dark, with a reason, for a missing or unknown theme", () => {
        const missing = resolveTheme("custom:gone", false, userThemes);
        expect(missing.render.themeClass).toBe("theme-dark");
        expect(missing.fallbackReason).toMatch(/gone/);

        const unknown = resolveTheme("Solarized", false, userThemes);
        expect(unknown.render.themeClass).toBe("theme-dark");
        expect(unknown.fallbackReason).toMatch(/Unknown theme/);
    });
});

describe("applyThemeRender", () => {
    let root: HTMLElement;
    beforeEach(() => {
        root = document.createElement("html");
        localStorage.clear();
    });

    it("switches the theme class without touching other classes", () => {
        root.className = "theme-dark keep-me";
        applyThemeRender(root, { themeClass: "theme-light", scheme: "light", tokens: {} });
        expect(root.classList.contains("theme-dark")).toBe(false);
        expect(root.classList.contains("theme-light")).toBe(true);
        expect(root.classList.contains("keep-me")).toBe(true);
    });

    it("sets user tokens and color-scheme, and removes them when switching to a built-in theme", () => {
        applyThemeRender(root, { themeClass: "theme-light", scheme: "light", tokens: custom.tokens });
        expect(root.style.getPropertyValue("--accent")).toBe("#ff007f");
        expect(root.style.getPropertyValue("--bg-app")).toBe("#fafafa");
        expect(root.style.getPropertyValue("color-scheme")).toBe("light");

        applyThemeRender(root, { themeClass: "theme-dark", scheme: "dark", tokens: {} });
        expect(root.style.getPropertyValue("--accent")).toBe("");
        expect(root.style.getPropertyValue("--bg-app")).toBe("");
        expect(root.style.getPropertyValue("color-scheme")).toBe("");
    });

    it("clears tokens of the previous user theme that the next one does not set", () => {
        applyThemeRender(root, { themeClass: "theme-dark", scheme: "dark", tokens: { "--accent": "#111", "--border": "#222" } });
        applyThemeRender(root, { themeClass: "theme-dark", scheme: "dark", tokens: { "--accent": "#333" } });
        expect(root.style.getPropertyValue("--accent")).toBe("#333");
        expect(root.style.getPropertyValue("--border")).toBe("");
    });

    it("cleans up properties set by the first-paint script", () => {
        // public/theme-boot.js records what it set in `data-theme-tokens`.
        root.style.setProperty("--accent", "#111");
        root.setAttribute("data-theme-tokens", "--accent");
        applyThemeRender(root, { themeClass: "theme-dark", scheme: "dark", tokens: {} });
        expect(root.style.getPropertyValue("--accent")).toBe("");
        expect(root.hasAttribute("data-theme-tokens")).toBe(false);
    });

    it("does not clear unrelated inline styles", () => {
        root.style.setProperty("--unrelated", "1");
        applyThemeRender(root, { themeClass: "theme-dark", scheme: "dark", tokens: { "--accent": "#111" } });
        applyThemeRender(root, { themeClass: "theme-dark", scheme: "dark", tokens: {} });
        expect(root.style.getPropertyValue("--unrelated")).toBe("1");
    });
});

describe("rememberTheme", () => {
    it("stores the resolved theme for the first-paint script", () => {
        const render = { themeClass: "theme-light", scheme: "light" as const, tokens: { "--accent": "#fff" } };
        rememberTheme(render);
        expect(JSON.parse(localStorage.getItem(THEME_STORAGE_KEY) ?? "null")).toEqual(render);
    });
});
