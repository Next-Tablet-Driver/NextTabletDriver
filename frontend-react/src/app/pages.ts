import type { FC } from "react";
import {
  Download01,
  Monitor01,
  PenTool01,
  Settings01,
  Sliders04,
  Terminal,
  Users01,
} from "@untitledui/icons";

export type PageId =
  "output" | "filters" | "penSettings" | "console" | "settings" | "release" | "credits";

export interface PageInfo {
  id: PageId;
  label: string;
  icon: FC<{ className?: string }>;
}

/** The pages of the main window, in the order of the navigation. */
export const PAGES: readonly PageInfo[] = [
  { id: "output", label: "Output", icon: Monitor01 },
  { id: "filters", label: "Filters", icon: Sliders04 },
  { id: "penSettings", label: "Pen Settings", icon: PenTool01 },
  { id: "console", label: "Console", icon: Terminal },
  { id: "settings", label: "Settings", icon: Settings01 },
  { id: "release", label: "Release", icon: Download01 },
  { id: "credits", label: "Credits", icon: Users01 },
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
