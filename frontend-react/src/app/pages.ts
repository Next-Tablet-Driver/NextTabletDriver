export type PageId =
  "output" | "filters" | "penSettings" | "console" | "settings" | "release" | "credits";

export interface PageInfo {
  id: PageId;
  label: string;
}

/** The pages of the main window, in the order of the tabs. */
export const PAGES: readonly PageInfo[] = [
  { id: "output", label: "Output" },
  { id: "filters", label: "Filters" },
  { id: "penSettings", label: "Pen Settings" },
  { id: "console", label: "Console" },
  { id: "settings", label: "Settings" },
  { id: "release", label: "Release" },
  { id: "credits", label: "Credits" },
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
