import { expect, test } from "./fixtures";

test.describe("Output page", () => {
  test.beforeEach(async ({ app }) => {
    await app.open();
  });

  test("shows the screen and the tablet geometry from the configuration", async ({ app }) => {
    await expect(app.content).toContainText("1920×1080");
    await expect(app.content).toContainText("224.00 × 148.00 mm");
    await expect(app.field("Width (px)")).toHaveValue("1920");
    await expect(app.field("Height (px)")).toHaveValue("1080");
    await expect(app.field("Width (mm)")).toHaveValue("224");
    await expect(app.field("Height (mm)")).toHaveValue("148");
  });

  test("editing the tablet area saves the new configuration and marks the profile modified", async ({
    app,
    page,
  }) => {
    await app.enter(app.field("Width (mm)"), "100");
    await app.enter(app.field("X (mm)"), "130");
    const saved = await app.savedConfigWhere(
      (config) => (config.active_area as { x?: number } | undefined)?.x === 130,
    );
    expect(saved.active_area).toMatchObject({ x: 130, w: 100 });
    await expect(page.getByText("Default Profile*")).toBeVisible();
  });

  test("the tablet area keeps its centre inside the tablet", async ({ app }) => {
    await app.enter(app.field("Width (mm)"), "100");
    await app.enter(app.field("X (mm)"), "999");
    // 224 mm wide tablet, 100 mm wide area: the centre cannot go past 224 - 100 / 2.
    await expect
      .poll(async () => (await app.lastSavedConfig()).active_area)
      .toMatchObject({ x: 174, w: 100 });
  });

  test("an area wider than the tablet is not applied", async ({ app }) => {
    await app.enter(app.field("Width (mm)"), "500");
    // The area keeps the full tablet width, and nothing is sent to the driver.
    await expect(app.field("Width (mm)")).toHaveValue("224");
    expect(await app.invocations("set_config")).toHaveLength(0);
  });

  test("keeps the proportions of the tablet area when the ratio is locked", async ({ app }) => {
    await app.enter(app.field("Width (mm)"), "112");
    await expect(app.field("Height (mm)")).toHaveValue("74");
  });

  test("a screen area is limited to the screens", async ({ app }) => {
    await app.enter(app.field("Width (px)"), "99999");
    await expect(app.field("Width (px)")).toHaveValue("1920");
  });

  test("the area of the tablet moves with the arrow keys", async ({ app, page }) => {
    await app.enter(app.field("Width (mm)"), "100");
    const area = page.getByRole("group", { name: /^Active area/ });
    await area.focus();
    await area.press("ArrowRight");
    await expect(app.field("X (mm)")).toHaveValue("112.5");
  });

  test("undo and redo with the keyboard", async ({ app, page }) => {
    const width = app.field("Width (mm)");
    await app.enter(width, "100");
    await app.savedConfigWhere(
      (config) => (config.active_area as { w?: number } | undefined)?.w === 100,
    );
    await page.keyboard.press("Control+z");
    await expect(width).toHaveValue("224");
    await page.keyboard.press("Control+y");
    await expect(width).toHaveValue("100");
  });

  test("Ctrl+S saves the profile", async ({ app, page }) => {
    await app.enter(app.field("Width (mm)"), "100");
    await expect(page.getByText("Default Profile*")).toBeVisible();
    await page.keyboard.press("Control+s");
    await expect.poll(async () => (await app.invocations("save_config")).length).toBe(1);
    await expect(page.getByText("Default Profile*")).toHaveCount(0);
  });

  test("switching to Relative mode is saved", async ({ app, page }) => {
    await page.getByRole("radio", { name: "Relative" }).click();
    expect((await app.savedConfigWhere((config) => config.mode === "Relative")).mode).toBe(
      "Relative",
    );
  });

  test("unchecking edge snapping is saved", async ({ app }) => {
    await app.toggle("Snap to the edges of the screens");
    expect(
      (await app.savedConfigWhere((config) => config.display_snapping === false)).display_snapping,
    ).toBe(false);
  });

  test("the configuration survives a page change", async ({ app }) => {
    await app.goTo("Pen Settings");
    await app.goTo("Output");
    await expect(app.field("Width (mm)")).toHaveValue("224");
  });
});
