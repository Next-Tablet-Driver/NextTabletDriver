import { describe, expect, it } from "vitest";
import { applyScheme, normalizeSelection, resolveScheme } from "@/theme/theme";

describe("resolveScheme", () => {
  it("follows the system for System, whatever the case", () => {
    expect(resolveScheme("System", true)).toBe("dark");
    expect(resolveScheme("system", false)).toBe("light");
  });

  it("honours Light and Dark", () => {
    expect(resolveScheme("Light", true)).toBe("light");
    expect(resolveScheme("Dark", false)).toBe("dark");
  });

  it("keeps the scheme of the themes this interface no longer has", () => {
    expect(resolveScheme("CatppuccinLatte", true)).toBe("light");
    expect(resolveScheme("CatppuccinMocha", false)).toBe("dark");
    expect(resolveScheme("custom:mine", false)).toBe("dark");
  });
});

describe("normalizeSelection", () => {
  it("maps any configuration value to one of the choices", () => {
    expect(normalizeSelection("system")).toBe("System");
    expect(normalizeSelection("CatppuccinLatte")).toBe("Light");
    expect(normalizeSelection("CatppuccinFrappe")).toBe("Dark");
    expect(normalizeSelection("custom:mine")).toBe("Dark");
  });
});

describe("applyScheme", () => {
  it("sets the class and the color scheme, and removes them again", () => {
    const root = document.createElement("html");
    applyScheme(root, "dark");
    expect(root.classList.contains("dark-mode")).toBe(true);
    expect(root.style.colorScheme).toBe("dark");

    applyScheme(root, "light");
    expect(root.classList.contains("dark-mode")).toBe(false);
    expect(root.style.colorScheme).toBe("light");
  });
});
