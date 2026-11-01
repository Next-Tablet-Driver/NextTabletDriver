import type { FC } from "react";
import { Code, Filter, Gears, Grid, Heart, Pen, Sparkles } from "@getpapillon/papicons";

export type PageId =
  "output" | "filters" | "penSettings" | "console" | "settings" | "release" | "credits";

export interface PageInfo {
  id: PageId;
  label: string;
  icon: FC<{ className?: string }>;
}

/** The pages of the main window, in the order of the tabs. */
export const PAGES: readonly PageInfo[] = [
  { id: "output", label: "Output", icon: Grid },
  { id: "filters", label: "Filters", icon: Filter },
  { id: "penSettings", label: "Pen Settings", icon: Pen },
  { id: "console", label: "Console", icon: Code },
  { id: "settings", label: "Settings", icon: Gears },
  { id: "release", label: "Release", icon: Sparkles },
  { id: "credits", label: "Credits", icon: Heart },
];

export const DEFAULT_PAGE: PageId = "output";

/** The URL fragment that shows a page: `#/output`. */
export function hrefOf(id: PageId): string {
  return `#/${id}`;
}

/** The page named by a URL fragment; the default page when it names none. */
export function pageFromHash(hash: string): PageId {
  const id = hash.replace(/^#\/?/, "");
  return PAGES.find((page) => page.id === id)?.id ?? DEFAULT_PAGE;
}
