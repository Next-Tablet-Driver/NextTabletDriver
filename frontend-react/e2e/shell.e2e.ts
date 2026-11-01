import { expect, TABS, test } from "./fixtures";

test.describe("application shell", () => {
  test("shows the version, the tablet and the profile once loaded", async ({ app, page }) => {
    await app.open();
    await expect(page.getByText("NextTabletDriver v2.0.0")).toBeVisible();
    await expect(app.footer).toContainText("Wacom Intuos Pro M");
    await expect(app.footer).toContainText("Default Profile");
    await expect(app.footer.getByRole("radio", { name: "Absolute" })).toBeChecked();
  });

  test("offers every page and opens each one without errors", async ({ app, page }) => {
    await app.open();
    for (const tab of TABS) {
      await app.goTo(tab);
      await expect(page).toHaveURL(new RegExp(`#/`));
      await expect(app.content.getByRole("heading", { level: 1 })).toBeVisible();
    }
  });

  test("starts on the Output page", async ({ app }) => {
    await app.open();
    await expect(app.content.getByRole("heading", { name: "Display" })).toBeVisible();
    await expect(app.content.getByRole("heading", { name: "Tablet", exact: true })).toBeVisible();
  });

  test("keeps the page across a reload", async ({ app, page }) => {
    await app.open();
    await app.goTo("Console");
    await page.reload();
    await expect(app.content.getByRole("heading", { name: "Console", level: 1 })).toBeVisible();
  });

  test("marks the current page in the navigation", async ({ app, page }) => {
    await app.open();
    await app.goTo("Settings");
    await expect(page.getByRole("tab", { name: "Settings", exact: true })).toHaveAttribute(
      "aria-selected",
      "true",
    );
  });

  test("shows the dark scheme by default", async ({ app, page }) => {
    await app.open();
    await expect(page.locator("html")).toHaveClass(/dark-mode/);
  });

  test("shows the light scheme when the configuration asks for it", async ({ app, page }) => {
    await app.open("&theme=Light");
    await expect(page.locator("html")).not.toHaveClass(/dark-mode/);
  });
});

test.describe("menu bar", () => {
  test("File menu lists the profile actions", async ({ app, page }) => {
    await app.open();
    await page.getByRole("button", { name: "File" }).click();
    for (const item of [
      "Load Settings...",
      "Save Settings",
      "Save Settings As...",
      "Reset to default",
      "Import OTD Settings",
    ]) {
      await expect(page.getByRole("menuitem", { name: item, exact: true })).toBeVisible();
    }
  });

  test("Save Settings asks the backend to save", async ({ app, page }) => {
    await app.open();
    await page.getByRole("button", { name: "File" }).click();
    await page.getByRole("menuitem", { name: "Save Settings", exact: true }).click();
    await expect.poll(async () => (await app.invocations("save_config")).length).toBe(1);
  });

  test("Reset to default asks the backend to reset and reloads the configuration", async ({
    app,
    page,
  }) => {
    await app.open();
    const before = (await app.invocations("get_config")).length;
    await page.getByRole("button", { name: "File" }).click();
    await page.getByRole("menuitem", { name: "Reset to default", exact: true }).click();
    await expect.poll(async () => (await app.invocations("reset_config")).length).toBe(1);
    await expect
      .poll(async () => (await app.invocations("get_config")).length)
      .toBeGreaterThan(before);
  });

  test("the menu closes with Escape", async ({ app, page }) => {
    await app.open();
    await page.getByRole("button", { name: "File" }).click();
    await expect(page.getByRole("menu")).toBeVisible();
    await page.keyboard.press("Escape");
    await expect(page.getByRole("menu")).toHaveCount(0);
  });

  test("Tablet menu offers the diagnostic windows", async ({ app, page }) => {
    await app.open();
    await page.getByRole("button", { name: "Tablet" }).click();
    await expect(page.getByRole("menuitem", { name: "Open Debugger" })).toBeVisible();
    await expect(page.getByRole("menuitem", { name: "Input Lag Analysis" })).toBeVisible();
  });

  test("Help menu links to the project", async ({ app, page }) => {
    await app.open();
    await page.getByRole("button", { name: "Help" }).click();
    await expect(page.getByRole("menuitem", { name: "Github Repository" })).toBeVisible();
  });

  test("Check for Updates tells the user when they are up to date", async ({ app, page }) => {
    await app.open();
    await page.getByRole("button", { name: "Help" }).click();
    await page.getByRole("menuitem", { name: "Check for Updates" }).click();
    await expect
      .poll(async () =>
        (await app.invocations("plugin:dialog|message")).map((call) => call.args.message),
      )
      .toContain("You are already on the latest version.");
  });
});

test.describe("narrow window", () => {
  test.use({ viewport: { width: 420, height: 800 } });

  test("keeps every page reachable: the tabs scroll sideways", async ({ app, page }) => {
    await app.open();
    const credits = page.getByRole("tab", { name: "Credits", exact: true });
    await credits.scrollIntoViewIfNeeded();
    await credits.click();
    await expect(app.content.getByRole("heading", { name: "Credits", level: 1 })).toBeVisible();
    // The page itself does not scroll sideways.
    expect(
      await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth),
    ).toBe(true);
  });
});
