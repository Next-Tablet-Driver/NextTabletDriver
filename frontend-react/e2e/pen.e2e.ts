import { expect, test } from "./fixtures";

test.describe("Pen Settings page", () => {
  test.beforeEach(async ({ app }) => {
    await app.open();
    await app.goTo("Pen Settings");
  });

  test("shows the current thresholds, switches and bindings", async ({ app }) => {
    await expect(app.field("Tip")).toHaveValue("10");
    await expect(app.content.getByRole("checkbox", { name: "Disable pressure" })).not.toBeChecked();
    await expect(app.content.getByRole("checkbox", { name: "Disable tilt" })).not.toBeChecked();
    await expect(app.field("Tip binding")).toHaveValue("Left Click");
    await expect(app.field("Eraser binding")).toHaveValue("Disabled");
    await expect(app.field("Pen button 1")).toHaveValue("Right Click");
    await expect(app.field("Pen button 2")).toHaveValue("Middle Click");
  });

  test("editing the tip threshold is saved", async ({ app }) => {
    await app.enter(app.field("Tip"), "25");
    expect(
      (await app.savedConfigWhere((config) => config.tip_threshold === 25)).tip_threshold,
    ).toBe(25);
  });

  test("disabling pressure is saved", async ({ app }) => {
    await app.content.getByText("Disable pressure").click();
    expect(
      (await app.savedConfigWhere((config) => config.disable_pressure === true)).disable_pressure,
    ).toBe(true);
  });

  test("changing the tip binding is saved", async ({ app }) => {
    await app.field("Tip binding").fill("Right Click");
    expect(
      (await app.savedConfigWhere((config) => config.tip_binding === "Right Click")).tip_binding,
    ).toBe("Right Click");
  });

  test("picking a preset sets the binding", async ({ app, page }) => {
    await app.content.getByRole("button", { name: "Eraser binding: presets" }).click();
    await page.getByRole("menuitem", { name: "Back", exact: true }).click();
    expect(
      (await app.savedConfigWhere((config) => config.eraser_binding === "Back")).eraser_binding,
    ).toBe("Back");
  });

  test("changing a pen button binding is saved without touching the others", async ({ app }) => {
    await app.field("Pen button 2").fill("Back");
    const saved = await app.savedConfigWhere(
      (config) =>
        JSON.stringify(config.pen_button_bindings) === JSON.stringify(["Right Click", "Back"]),
    );
    expect(saved.pen_button_bindings).toEqual(["Right Click", "Back"]);
  });

  test("lists one field per key of the tablet, with the saved bindings", async ({ app }) => {
    await expect(app.content.getByRole("heading", { name: "Tablet keys" })).toBeVisible();
    await expect(app.field("Tablet key 1")).toHaveValue("Ctrl+Z");
    await expect(app.field("Tablet key 2")).toHaveValue("Space");
    await expect(app.field("Tablet key 4")).toHaveValue("None");
    await expect(app.field("Tablet key 5")).toHaveCount(0);
  });

  test("changing a tablet key binding is saved without touching the others", async ({ app }) => {
    await app.field("Tablet key 3").fill("F5");
    const saved = await app.savedConfigWhere(
      (config) =>
        JSON.stringify(config.aux_button_bindings) ===
        JSON.stringify(["Ctrl+Z", "Space", "F5", "None"]),
    );
    expect(saved.aux_button_bindings).toEqual(["Ctrl+Z", "Space", "F5", "None"]);
  });

  test("recording a key combination binds it", async ({ app, page }) => {
    await app.content.getByRole("button", { name: "Record Tablet key 4" }).click();
    await page.keyboard.down("Control");
    await page.keyboard.press("KeyS");
    await page.keyboard.up("Control");
    await expect(app.field("Tablet key 4")).toHaveValue("Ctrl+S");
    const saved = await app.savedConfigWhere(
      (config) => (config.aux_button_bindings as string[] | undefined)?.[3] === "Ctrl+S",
    );
    expect(saved.aux_button_bindings).toEqual(["Ctrl+Z", "Space", "None", "Ctrl+S"]);
  });

  test("a text that is not a binding is flagged with the driver's message", async ({ app }) => {
    const field = app.field("Pen button 1");
    await field.fill("Banana");
    await field.press("Tab");
    await expect(app.content.getByRole("alert")).toContainText("Unknown key or button");
    await expect(field).toHaveAttribute("aria-invalid", "true");
  });

  test("selecting a pressure curve marks it and is saved", async ({ app }) => {
    const exponential = app.content.getByRole("radio", { name: "Exponential" });
    await exponential.click();
    await expect(exponential).toBeChecked();
    await expect(app.content.getByRole("radio", { name: "Linear" })).not.toBeChecked();
    const saved = await app.savedConfigWhere(
      (config) =>
        (config.pressure_curve as { curve_type?: string } | undefined)?.curve_type ===
        "Exponential",
    );
    expect(saved.pressure_curve).toMatchObject({ curve_type: "Exponential" });
  });

  test("an exponential curve can be tuned and is drawn", async ({ app, page }) => {
    await app.content.getByRole("radio", { name: "Exponential" }).click();
    await app.enter(app.field("Exponent"), "2");
    await expect(page.getByRole("img", { name: /Pressure curve, Exponential/ })).toBeVisible();
    const saved = await app.savedConfigWhere(
      (config) => (config.pressure_curve as { exponent?: number } | undefined)?.exponent === 2,
    );
    expect(saved.pressure_curve).toMatchObject({ exponent: 2 });
  });

  test("a custom curve lists its control points and takes new ones", async ({ app }) => {
    await expect(app.field("Point 2: raw")).toHaveValue("0.25");
    await expect(app.field("Point 1: raw")).toBeDisabled();
    await app.content.getByRole("button", { name: "Add a point" }).click();
    const saved = await app.savedConfigWhere(
      (config) =>
        ((config.pressure_curve as { points?: unknown[] } | undefined)?.points?.length ?? 0) === 6,
    );
    expect(saved.pressure_curve).toMatchObject({ curve_type: "Custom" });
  });

  test("a control point can be removed, but not the ends", async ({ app }) => {
    await expect(app.content.getByRole("button", { name: "Remove point 1" })).toBeDisabled();
    await app.content.getByRole("button", { name: "Remove point 3" }).click();
    await app.savedConfigWhere(
      (config) =>
        ((config.pressure_curve as { points?: unknown[] } | undefined)?.points?.length ?? 0) === 4,
    );
  });

  test("the profile is marked modified after a change", async ({ app, page }) => {
    await app.content.getByText("Disable tilt").click();
    await expect(page.getByText("Default Profile*")).toBeVisible();
  });
});
