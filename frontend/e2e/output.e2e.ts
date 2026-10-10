import { expect, test } from "./fixtures";

test.describe("Output screen", () => {
    test.beforeEach(async ({ app }) => {
        await app.open();
    });

    test("shows the screen and the tablet geometry from the configuration", async ({ app }) => {
        await expect(app.content).toContainText("1920x1080");
        await expect(app.content).toContainText("224.00mm");
        await expect(app.content).toContainText("148.00mm");
        const inputs = app.content.locator("input[type=number]");
        await expect(inputs).toHaveCount(9);
        await expect(inputs.nth(0)).toHaveValue("1920");
        await expect(inputs.nth(1)).toHaveValue("1080");
        await expect(inputs.nth(4)).toHaveValue("224.00");
        await expect(inputs.nth(5)).toHaveValue("148.00");
    });

    test("editing the tablet area saves the new configuration and marks the profile modified", async ({ app, page }) => {
        const inputs = app.content.locator("input[type=number]");
        await app.enter(inputs.nth(4), "100");
        await app.enter(inputs.nth(6), "130");
        const saved = await app.savedConfigWhere((config) => (config.active_area as { x?: number } | undefined)?.x === 130);
        expect(saved.active_area).toMatchObject({ x: 130, w: 100 });
        await expect(page.getByText("Default Profile*")).toBeVisible();
    });

    test("the tablet area keeps its centre inside the tablet", async ({ app }) => {
        const inputs = app.content.locator("input[type=number]");
        await app.enter(inputs.nth(4), "100");
        await app.enter(inputs.nth(6), "999");
        const saved = await app.savedConfigWhere((config) => (config.active_area as { w?: number } | undefined)?.w === 100);
        // 224 mm wide tablet, 100 mm wide area: the centre cannot go past 224 - 100 / 2.
        await expect.poll(async () => (await app.lastSavedConfig()).active_area).toMatchObject({ x: 174 });
        expect(saved.active_area).toMatchObject({ w: 100 });
    });

    test("an area wider than the tablet is not applied", async ({ app }) => {
        const width = app.content.locator("input[type=number]").nth(4);
        await app.enter(width, "500");
        // The configuration keeps the full tablet width, whatever the field still displays.
        await expect(app.content).toContainText("224.00mm");
        expect(await app.invocations("set_config")).toHaveLength(0);
    });

    test("undo and redo with the keyboard", async ({ app, page }) => {
        const width = app.content.locator("input[type=number]").nth(4);
        await app.enter(width, "100");
        await app.savedConfigWhere((config) => (config.active_area as { w?: number } | undefined)?.w === 100);
        await page.locator("main").press("Control+z");
        await expect(width).toHaveValue(/^224/);
        await page.locator("main").press("Control+y");
        await expect(width).toHaveValue(/^100/);
    });

    test("Ctrl+S saves the profile", async ({ app, page }) => {
        const inputs = app.content.locator("input[type=number]");
        await app.enter(inputs.nth(4), "100");
        await expect(page.getByText("Default Profile*")).toBeVisible();
        await page.locator("main").press("Control+s");
        await expect.poll(async () => (await app.invocations("save_config")).length).toBe(1);
        await expect(page.getByText("Default Profile*")).toHaveCount(0);
    });

    test("switching to Relative mode is saved", async ({ app, page }) => {
        await page.getByRole("button", { name: "Absolute Mode" }).click();
        await page.getByRole("option", { name: "Relative Mode" }).click();
        expect((await app.savedConfigWhere((config) => config.mode === "Relative")).mode).toBe("Relative");
    });

    test("unchecking edge snapping is saved", async ({ app }) => {
        await app.content.getByLabel("Enable Edge Snapping").uncheck();
        expect((await app.savedConfigWhere((config) => config.display_snapping === false)).display_snapping).toBe(false);
    });

    test("the configuration survives a screen change", async ({ app }) => {
        await app.goTo("Pen Settings");
        await app.goTo("Output");
        const inputs = app.content.locator("input[type=number]");
        await expect(inputs).toHaveCount(9);
        await expect(inputs.nth(4)).toHaveValue("224.00");
    });
});
