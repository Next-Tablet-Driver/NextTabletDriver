import { useEffect } from "react";
import { applyScheme, resolveScheme } from "@/theme/theme";

const DARK_QUERY = "(prefers-color-scheme: dark)";

/** Keeps the document in the scheme of `selection`, and follows the system when it says so. */
export function useTheme(selection: string): void {
  useEffect(() => {
    const media = typeof window.matchMedia === "function" ? window.matchMedia(DARK_QUERY) : null;
    const apply = (): void => {
      applyScheme(document.documentElement, resolveScheme(selection, media?.matches ?? true));
    };
    apply();
    media?.addEventListener("change", apply);
    return () => {
      media?.removeEventListener("change", apply);
    };
  }, [selection]);
}
