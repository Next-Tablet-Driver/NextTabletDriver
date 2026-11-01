/**
 * The look of the application is Untitled UI's: a light and a dark scheme, switched by the
 * `dark-mode` class on `<html>`. The configuration keeps a `theme` string; this module turns it
 * into one of the two schemes.
 *
 * Themes of the Svelte interface that no longer exist here (the Catppuccin palettes, the user
 * theme files) keep their scheme: they are shown as the dark or the light scheme.
 */

export type Scheme = "light" | "dark";

/** What the user can choose in Settings; also the values written in the configuration. */
export const THEME_CHOICES = [
  { value: "System", label: "System" },
  { value: "Dark", label: "Dark" },
  { value: "Light", label: "Light" },
] as const;

const LIGHT_NAMES = new Set(["light", "catppuccinlatte"]);

/** The scheme to show for a configuration value (`System` follows the operating system). */
export function resolveScheme(selection: string, prefersDark: boolean): Scheme {
  const lowered = selection.toLowerCase();
  if (lowered === "system") {
    return prefersDark ? "dark" : "light";
  }
  return LIGHT_NAMES.has(lowered) ? "light" : "dark";
}

/** The value of `THEME_CHOICES` that stands for a configuration value that is not one of them. */
export function normalizeSelection(selection: string): string {
  const lowered = selection.toLowerCase();
  if (lowered === "system") {
    return "System";
  }
  return LIGHT_NAMES.has(lowered) ? "Light" : "Dark";
}

/** Writes a scheme on the document. */
export function applyScheme(root: HTMLElement, scheme: Scheme): void {
  root.classList.toggle("dark-mode", scheme === "dark");
  root.style.colorScheme = scheme;
}
