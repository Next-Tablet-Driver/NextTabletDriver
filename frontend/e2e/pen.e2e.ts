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
        await expect(app.content.getByRole("textbox").nth(0)).toHaveValue("Left Click");
        await expect(app.content.getByRole("textbox").nth(1)).toHaveValue("Disabled");
        await expect(app.content.getByLabel("Pen Button 1")).toHaveValue("Right Click");
        await expect(app.content.getByLabel("Pen Button 2")).toHaveValue("Middle Click");
    });

    test("editing the tip threshold is saved", async ({ app }) => {
        await app.content.locator("input[type=number]").first().fill("25");
        expect((await app.savedConfigWhere((config) => config.tip_threshold === 25)).tip_threshold).toBe(25);
    });

    test("disabling pressure is saved", async ({ app }) => {
        await app.content.getByLabel("Disable Pressure").check();
        expect((await app.savedConfigWhere((config) => config.disable_pressure === true)).disable_pressure).toBe(true);
    });

    test("changing the tip binding is saved", async ({ app }) => {
        await app.content.getByRole("textbox").nth(0).fill("Right Click");
        expect((await app.savedConfigWhere((config) => config.tip_binding === "Right Click")).tip_binding).toBe("Right Click");
    });

    test("changing a pen button binding is saved without touching the others", async ({ app }) => {
        await app.content.getByLabel("Pen Button 2").fill("Back");
        const saved = await app.savedConfigWhere((config) => JSON.stringify(config.pen_button_bindings) === JSON.stringify(["Right Click", "Back"]));
        expect(saved.pen_button_bindings).toEqual(["Right Click", "Back"]);
    });

    test("selecting a pressure curve highlights it and is saved", async ({ app }) => {
        await app.content.getByRole("button", { name: "Exponential" }).click();
        await expect(app.content.getByRole("button", { name: "Exponential" })).toHaveClass(/primary/);
        await expect(app.content.getByRole("button", { name: "Custom" })).toHaveClass(/ghost/);
        const saved = await app.savedConfigWhere((config) => (config.pressure_curve as { curve_type?: string } | undefined)?.curve_type === "Exponential");
        expect(saved.pressure_curve).toMatchObject({ curve_type: "Exponential" });
    });

    test("the profile is marked modified after a change", async ({ app, page }) => {
        await app.content.getByLabel("Disable Tilt").check();
        await expect(page.getByText("Default Profile*")).toBeVisible();
    });
});
