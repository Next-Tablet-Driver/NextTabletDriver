import { expect, test } from "./fixtures";

test.describe("Pen Settings screen", () => {
    test.beforeEach(async ({ app }) => {
        await app.open();
        await app.goTo("Pen Settings");
    });

    test("shows the current thresholds, switches and bindings", async ({ app }) => {
        await expect(app.content.locator("input[type=number]").first()).toHaveValue("10");
        await expect(app.content.getByLabel("Disable Pressure")).not.toBeChecked();
        await expect(app.content.getByLabel("Disable Tilt")).not.toBeChecked();
        await expect(app.content.getByLabel("Tip binding", { exact: true })).toHaveValue("Left Click");
        await expect(app.content.getByLabel("Eraser binding", { exact: true })).toHaveValue("Disabled");
        await expect(app.content.getByLabel("Pen Button 1", { exact: true })).toHaveValue("Right Click");
        await expect(app.content.getByLabel("Pen Button 2", { exact: true })).toHaveValue("Middle Click");
    });

    test("editing the tip threshold is saved", async ({ app }) => {
        await app.content.locator("input[type=number]").first().fill("25");
        expect((await app.savedConfigWhere((config) => config.tip_threshold === 25)).tip_threshold).toBe(25);
    });

    test("disabling pressure is saved", async ({ app }) => {
        await app.content.getByText("Disable Pressure").click();
        expect((await app.savedConfigWhere((config) => config.disable_pressure === true)).disable_pressure).toBe(true);
    });

    test("changing the tip binding is saved", async ({ app }) => {
        await app.content.getByLabel("Tip binding", { exact: true }).fill("Right Click");
        expect((await app.savedConfigWhere((config) => config.tip_binding === "Right Click")).tip_binding).toBe("Right Click");
    });

    test("changing a pen button binding is saved without touching the others", async ({ app }) => {
        await app.content.getByLabel("Pen Button 2", { exact: true }).fill("Back");
        const saved = await app.savedConfigWhere((config) => JSON.stringify(config.pen_button_bindings) === JSON.stringify(["Right Click", "Back"]));
        expect(saved.pen_button_bindings).toEqual(["Right Click", "Back"]);
    });

    test("lists one field per key of the tablet, with the saved bindings", async ({ app }) => {
        await expect(app.content.getByText("Tablet Keys")).toBeVisible();
        await expect(app.content.getByLabel("Tablet Key 1", { exact: true })).toHaveValue("Ctrl+Z");
        await expect(app.content.getByLabel("Tablet Key 2", { exact: true })).toHaveValue("Space");
        await expect(app.content.getByLabel("Tablet Key 4", { exact: true })).toHaveValue("None");
        await expect(app.content.getByLabel("Tablet Key 5", { exact: true })).toHaveCount(0);
    });

    test("changing a tablet key binding is saved without touching the others", async ({ app }) => {
        await app.content.getByLabel("Tablet Key 3", { exact: true }).fill("F5");
        const saved = await app.savedConfigWhere((config) => JSON.stringify(config.aux_button_bindings) === JSON.stringify(["Ctrl+Z", "Space", "F5", "None"]));
        expect(saved.aux_button_bindings).toEqual(["Ctrl+Z", "Space", "F5", "None"]);
    });

    test("recording a key combination binds it", async ({ app, page }) => {
        await app.content.getByRole("button", { name: "Record Tablet Key 4" }).click();
        await page.keyboard.down("Control");
        await page.keyboard.press("KeyS");
        await page.keyboard.up("Control");
        await expect(app.content.getByLabel("Tablet Key 4", { exact: true })).toHaveValue("Ctrl+S");
        const saved = await app.savedConfigWhere((config) => (config.aux_button_bindings as string[] | undefined)?.[3] === "Ctrl+S");
        expect(saved.aux_button_bindings).toEqual(["Ctrl+Z", "Space", "None", "Ctrl+S"]);
    });

    test("a text that is not a binding is flagged with the driver's message", async ({ app }) => {
        const field = app.content.getByLabel("Pen Button 1", { exact: true });
        await field.fill("Banana");
        await field.press("Tab");
        await expect(app.content.getByRole("alert")).toContainText("Unknown key or button");
        await expect(field).toHaveAttribute("aria-invalid", "true");
    });

    test("selecting a pressure curve highlights it and is saved", async ({ app }) => {
        await app.content.getByRole("button", { name: "Exponential" }).click();
        await expect(app.content.getByRole("button", { name: "Exponential" })).toHaveClass(/primary/);
        await expect(app.content.getByRole("button", { name: "Custom" })).toHaveClass(/ghost/);
        const saved = await app.savedConfigWhere((config) => (config.pressure_curve as { curve_type?: string } | undefined)?.curve_type === "Exponential");
        expect(saved.pressure_curve).toMatchObject({ curve_type: "Exponential" });
    });

    test("the profile is marked modified after a change", async ({ app, page }) => {
        await app.content.getByText("Disable Tilt").click();
        await expect(page.getByText("Default Profile*")).toBeVisible();
    });
});
