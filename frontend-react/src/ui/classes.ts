/** Joins the class names that are set; the rest (`false`, `undefined`, `""`) is left out. */
export function classes(...parts: (string | false | null | undefined)[]): string {
  return parts.filter((part): part is string => typeof part === "string" && part !== "").join(" ");
}
