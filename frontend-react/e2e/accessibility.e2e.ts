import AxeBuilder from "@axe-core/playwright";
import { expect, TABS, test } from "./fixtures";

// An automatic audit finds what a tool can: missing names, contrast, roles. It does not replace
// a keyboard-only pass, which the area editors and the bindings have (see their own specs).
for (const scheme of ["Dark", "Light"]) {
  test.describe(`accessibility, ${scheme} scheme`, () => {
    for (const tab of TABS) {
      test(`${tab} has no serious violation`, async ({ app, page }) => {
        await app.open(`&theme=${scheme}`);
        await app.goTo(tab);
        await expect(app.content.getByRole("heading", { level: 1 })).toBeVisible();
        // Let the data of the page arrive before looking at it.
        await page.waitForTimeout(600);
        const { violations } = await new AxeBuilder({ page })
          .withTags(["wcag2a", "wcag2aa", "wcag21a", "wcag21aa"])
          .analyze();
        const serious = violations.filter((v) => v.impact === "serious" || v.impact === "critical");
        expect(
          serious.map(
            (v) => `${v.id}: ${v.help}: ${v.nodes.map((n) => n.target.join(" ")).join(" | ")}`,
          ),
        ).toEqual([]);
      });
    }
  });
}
