import { expect, test } from "./fixtures";

type PluginsConfig = Record<string, { enabled?: boolean; properties?: Record<string, unknown> }> | undefined;

test.describe("Filters screen", () => {
    test.beforeEach(async ({ app }) => {
        await app.open();
        await app.goTo("Filters");
    });

    test("lists the available filters and shows the first one", async ({ app }) => {
        await expect(app.content).toContainText("AVAILABLE FILTERS");
        await expect(app.content.getByRole("button", { name: "Kalman Smoothing" })).toBeVisible();
        await expect(app.content.getByRole("button", { name: "Hand Speed" })).toBeVisible();
        await expect(app.content).toContainText("v1.0.0");
        await expect(app.content).toContainText("By NextTablet");
    });

    test("selecting another filter shows its details", async ({ app }) => {
        await app.content.getByRole("button", { name: "Hand Speed" }).click();
        await expect(app.content).toContainText("Tracks hand speed and travel distance.");
        await expect(app.content).toContainText("v1.2.0");
        await expect(app.content).toContainText("By nzbasic");
    });

    test("renders one control per setting of a filter", async ({ app }) => {
        await expect(app.content).toContainText("Process noise");
        await expect(app.content).toContainText("Latency");
        await expect(app.content).toContainText("Invert");
        const numbers = app.content.locator("input[type=number]");
        await expect(numbers).toHaveCount(2);
        await expect(numbers.nth(0)).toHaveValue("0.5");
        await expect(numbers.nth(1)).toHaveValue("4");
        await expect(app.content.locator("input[type=checkbox]").nth(1)).not.toBeChecked();
        await expect(app.content.getByRole("button", { name: "Balanced" })).toBeVisible();
    });

    test("editing a setting is saved in the configuration of that filter", async ({ app }) => {
        await app.enter(app.content.locator("input[type=number]").nth(1), "9");
        const saved = await app.savedConfigWhere((config) => (config.plugins as PluginsConfig)?.kalman_smoothing?.properties?.latency === 9);
        expect(saved.plugins).toMatchObject({ kalman_smoothing: { enabled: true, properties: { latency: 9, process_noise: 0.5 } } });
    });

    test("disabling a filter is saved", async ({ app }) => {
        await app.content.getByLabel("Enable Plugin").uncheck();
        const saved = await app.savedConfigWhere((config) => (config.plugins as PluginsConfig)?.kalman_smoothing?.enabled === false);
        expect(saved.plugins).toMatchObject({ kalman_smoothing: { enabled: false } });
    });

    test("choosing another mode in a choice setting is saved", async ({ app, page }) => {
        await app.content.getByRole("button", { name: "Balanced" }).click();
        await page.getByRole("option", { name: "Smooth" }).click();
        await expect(app.content.getByRole("button", { name: "Smooth", exact: true })).toBeVisible();
        // A choice is stored as the index of the option: "Smooth" is the third one.
        const saved = await app.savedConfigWhere((config) => (config.plugins as PluginsConfig)?.kalman_smoothing?.properties?.mode === 2);
        expect(saved.plugins).toMatchObject({ kalman_smoothing: { properties: { mode: 2, latency: 4 } } });
    });

    test("warns about libraries that were not installed through the app", async ({ app }) => {
        const alert = app.content.getByRole("alert");
        await expect(alert).toContainText("2 plugins were not loaded");
        await expect(alert).toContainText("mystery_filter.dll");
        await expect(alert).toContainText("mystery_filter - Copy.dll");
    });

    test("trusting a library sends its hash and removes the warning", async ({ app }) => {
        await app.content.getByRole("button", { name: "Review and trust" }).first().click();
        const calls = await app.invocations("trust_plugin");
        expect(calls).toHaveLength(1);
        expect(calls[0]?.args.sha256).toBe("a".repeat(64));
        await expect(app.content.getByRole("alert")).toHaveCount(0);
    });

    test("reloading the filters asks the backend", async ({ app }) => {
        await app.content.getByTitle("Reload Plugins").click();
        await expect.poll(async () => (await app.invocations("reload_plugins")).length).toBe(1);
    });

    test("the plugins folder can be opened", async ({ app }) => {
        await app.content.getByTitle("Open Plugins Folder").click();
        await expect.poll(async () => (await app.invocations("open_plugins_folder")).length).toBe(1);
    });
});
