import { expect, test } from "./fixtures";

test.describe("Settings screen", () => {
    test.beforeEach(async ({ app }) => {
        await app.open();
        await app.goTo("Settings");
    });

    test("reflects the general options of the configuration", async ({ app }) => {
        await expect(app.content.getByLabel("Run at startup")).not.toBeChecked();
        await expect(app.content.getByLabel("System Tray when Minimize")).toBeChecked();
        await expect(app.content.getByLabel("Force High Resolution Timer (0.5ms)")).toBeChecked();
    });

    test("a general option is saved when toggled", async ({ app }) => {
        await app.content.getByLabel("Run at startup").check();
        expect((await app.savedConfigWhere((config) => config.run_at_startup === true)).run_at_startup).toBe(true);
    });

    test("lists the user themes and reports the ones that cannot be loaded", async ({ app }) => {
        await expect(app.content).toContainText("Themes that could not be loaded");
        await expect(app.content).toContainText("broken.json");
        await expect(app.content).toContainText("tokens.accent expected a color");
    });

    test("the theme picker offers the built-in and the user themes", async ({ app, page }) => {
        await app.content.getByRole("button", { name: "Dark", exact: true }).click();
        await expect(page.getByRole("option", { name: "Light" })).toBeVisible();
        await expect(page.getByRole("option", { name: "Neon Night" })).toBeVisible();
    });

    test("choosing a user theme applies it and saves the choice", async ({ app, page }) => {
        await app.content.getByRole("button", { name: "Dark", exact: true }).click();
        await page.getByRole("option", { name: "Neon Night" }).click();
        expect((await app.savedConfigWhere((config) => config.theme !== "Dark")).theme).not.toBe("Dark");
        await expect.poll(() => page.evaluate(() => getComputedStyle(document.documentElement).getPropertyValue("--accent").trim().toLowerCase())).toBe("#ff2bd6");
    });

    test("the themes folder can be opened", async ({ app }) => {
        await app.content.getByRole("button", { name: "Open Themes Folder" }).click();
        await expect.poll(async () => (await app.invocations("open_themes_folder")).length).toBe(1);
    });

    test("the theme list can be reloaded", async ({ app }) => {
        const before = (await app.invocations("list_themes")).length;
        await app.content.getByRole("button", { name: "Reload" }).click();
        await expect.poll(async () => (await app.invocations("list_themes")).length).toBeGreaterThan(before);
    });

    test("shows the language and the WebSocket server state", async ({ app }) => {
        await expect(app.content.getByRole("button", { name: "English", exact: true })).toBeVisible();
        await expect(app.content).toContainText("RUNNING");
        await expect(app.content.locator("input[type=number]").nth(0)).toHaveValue("8765");
    });

    test("stopping the WebSocket server is saved and updates the state", async ({ app }) => {
        await app.content.getByLabel("Enable WebSocket Server").uncheck();
        await expect(app.content).toContainText("STOPPED");
        const saved = await app.savedConfigWhere((config) => (config.websocket as { enabled?: boolean } | undefined)?.enabled === false);
        expect(saved.websocket).toMatchObject({ enabled: false, port: 8765 });
    });

    test("changing the WebSocket port is saved", async ({ app }) => {
        await app.enter(app.content.locator("input[type=number]").nth(0), "9100");
        const saved = await app.savedConfigWhere((config) => (config.websocket as { port?: number } | undefined)?.port === 9100);
        expect(saved.websocket).toMatchObject({ port: 9100, polling_rate_hz: 60 });
    });

    test("the WebSocket payload can be chosen", async ({ app }) => {
        await app.content.getByLabel("Tilt").check();
        const saved = await app.savedConfigWhere((config) => (config.websocket as { send_tilt?: boolean } | undefined)?.send_tilt === true);
        expect(saved.websocket).toMatchObject({ send_tilt: true, send_pressure: true });
    });
});
