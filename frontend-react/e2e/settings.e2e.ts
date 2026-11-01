import { expect, test } from "./fixtures";

test.describe("Settings page", () => {
  test.beforeEach(async ({ app }) => {
    await app.open();
    await app.goTo("Settings");
  });

  test("reflects the general options of the configuration", async ({ app }) => {
    await expect(app.content.getByRole("switch", { name: "Run at startup" })).not.toBeChecked();
    await expect(
      app.content.getByRole("switch", { name: "Hide to the system tray when minimized" }),
    ).toBeChecked();
    await expect(
      app.content.getByRole("switch", { name: "Force high resolution timer (0.5 ms)" }),
    ).toBeChecked();
  });

  test("a general option is saved when toggled", async ({ app }) => {
    await app.content.getByText("Run at startup").click();
    expect(
      (await app.savedConfigWhere((config) => config.run_at_startup === true)).run_at_startup,
    ).toBe(true);
  });

  test("the theme picker offers the system, dark and light schemes", async ({ app, page }) => {
    await app.content.getByRole("button", { name: /Theme/ }).click();
    await expect(page.getByRole("option", { name: "System" })).toBeVisible();
    await expect(page.getByRole("option", { name: "Dark" })).toBeVisible();
    await expect(page.getByRole("option", { name: "Light" })).toBeVisible();
  });

  test("choosing the light scheme applies it and saves the choice", async ({ app, page }) => {
    await app.content.getByRole("button", { name: /Theme/ }).click();
    await page.getByRole("option", { name: "Light" }).click();
    await expect(page.locator("html")).not.toHaveClass(/dark-mode/);
    expect((await app.savedConfigWhere((config) => config.theme === "Light")).theme).toBe("Light");
  });

  test("shows the language and the WebSocket server state", async ({ app }) => {
    await expect(app.content.getByRole("button", { name: /Interface language/ })).toContainText(
      "English",
    );
    await expect(app.content).toContainText("Running");
    await expect(app.field("Port")).toHaveValue("8765");
  });

  test("stopping the WebSocket server is saved and updates the state", async ({ app }) => {
    await app.content.getByText("Enable the WebSocket server").click();
    await expect(app.content).toContainText("Stopped");
    const saved = await app.savedConfigWhere(
      (config) => (config.websocket as { enabled?: boolean } | undefined)?.enabled === false,
    );
    expect(saved.websocket).toMatchObject({ enabled: false, port: 8765 });
  });

  test("changing the WebSocket port is saved", async ({ app }) => {
    await app.enter(app.field("Port"), "9100");
    const saved = await app.savedConfigWhere(
      (config) => (config.websocket as { port?: number } | undefined)?.port === 9100,
    );
    expect(saved.websocket).toMatchObject({ port: 9100, polling_rate_hz: 60 });
  });

  test("the WebSocket payload can be chosen", async ({ app }) => {
    await app.content.getByText("Tilt", { exact: true }).click();
    const saved = await app.savedConfigWhere(
      (config) => (config.websocket as { send_tilt?: boolean } | undefined)?.send_tilt === true,
    );
    expect(saved.websocket).toMatchObject({ send_tilt: true, send_pressure: true });
  });
});
