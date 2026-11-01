import { describe, expect, it } from "vitest";
import { DEFAULT_PAGE, PAGES, hrefOf, pageFromHash } from "@/app/pages";

describe("pages", () => {
  it("round-trips every page through its URL fragment", () => {
    for (const page of PAGES) {
      expect(pageFromHash(hrefOf(page.id))).toBe(page.id);
    }
  });

  it("shows the default page for an empty or unknown fragment", () => {
    expect(pageFromHash("")).toBe(DEFAULT_PAGE);
    expect(pageFromHash("#/nowhere")).toBe(DEFAULT_PAGE);
  });

  it("accepts a fragment without the slash", () => {
    expect(pageFromHash("#console")).toBe("console");
  });
});
