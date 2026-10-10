import { expect, TABS, test } from "./fixtures";

test.describe("application shell", () => {
    test("shows the version, the tablet and the profile once loaded", async ({ app, page }) => {
        await app.open();
        await expect(page.getByText("NextTabletDriver v2.0.0")).toBeVisible();
        await expect(app.footer).toContainText("Wacom Intuos Pro M");
        await expect(app.footer).toContainText("Default Profile");
        await expect(app.footer).toContainText("Absolute Mode");
    });

    test("offers every screen and opens each one without errors", async ({ app, page }) => {
        await app.open();
        for (const tab of TABS) {
            await app.goTo(tab);
            await expect(app.content).not.toBeEmpty();
            await expect(page.getByText("under construction")).toHaveCount(0);
        }
    });

    test("starts on the Output screen", async ({ app }) => {
        await app.open();
        await expect(app.content).toContainText("Display");
        await expect(app.content).toContainText("Tablet");
    });

    test("applies the configured theme to the document", async ({ app, page }) => {
        await app.open();
        const theme = await page.evaluate(() => document.documentElement.dataset.theme ?? document.documentElement.className);
        expect(theme.toLowerCase()).toContain("dark");
    });

    test("can run with the light theme", async ({ app, page }) => {
        await app.open("&theme=Light");
        const theme = await page.evaluate(() => document.documentElement.dataset.theme ?? document.documentElement.className);
        expect(theme.toLowerCase()).toContain("light");
    });
});

test.describe("menu bar", () => {
    test("File menu lists the profile actions", async ({ app, page }) => {
        await app.open();
        await page.getByRole("button", { name: "File" }).click();
        for (const item of ["Load Settings...", "Save Settings", "Save Settings As...", "Reset to default", "Import OTD Settings"]) {
            await expect(page.getByText(item, { exact: true })).toBeVisible();
        }
    });

    test("Save Settings asks the backend to save", async ({ app, page }) => {
        await app.open();
        await page.getByRole("button", { name: "File" }).click();
        await page.getByText("Save Settings", { exact: true }).click();
        expect(await app.invocations("save_config")).toHaveLength(1);
    });

    test("Reset to default asks the backend to reset and reloads the configuration", async ({ app, page }) => {
        await app.open();
        const before = (await app.invocations("get_config")).length;
        await page.getByRole("button", { name: "File" }).click();
        await page.getByText("Reset to default", { exact: true }).click();
        await expect.poll(async () => (await app.invocations("reset_config")).length).toBe(1);
        await expect.poll(async () => (await app.invocations("get_config")).length).toBeGreaterThan(before);
    });

    test("the menu closes when clicking elsewhere", async ({ app, page }) => {
        await app.open();
        await page.getByRole("button", { name: "File" }).click();
        await expect(page.getByText("Save Settings", { exact: true })).toBeVisible();
        await app.content.click({ position: { x: 900, y: 500 } });
        await expect(page.getByText("Save Settings", { exact: true })).toHaveCount(0);
    });

    test("Tablet menu offers the diagnostic windows", async ({ app, page }) => {
        await app.open();
        await page.getByRole("button", { name: "Tablet" }).click();
        await expect(page.getByText("Open Debugger")).toBeVisible();
        await expect(page.getByText("Input Lag Analysis")).toBeVisible();
    });

    test("Help menu links to the project and offers the bug and feature forms", async ({ app, page }) => {
        await app.open();
        await page.getByRole("button", { name: "Help" }).click();
        await expect(page.getByText("Github Repository")).toBeVisible();
        await page.getByText("Report Bug / Suggest Feature").hover();
        await expect(page.getByText("Report a Bug")).toBeVisible();
        await expect(page.getByText("Suggest a Feature")).toBeVisible();
    });

    test("Check for Update tells the user when they are up to date", async ({ app, page }) => {
        await app.open();
        await page.getByRole("button", { name: "Help" }).click();
        await page.getByText("Check for Update").click();
        await expect
            .poll(async () => (await app.invocations("plugin:dialog|message")).map((call) => call.args.message))
            .toContain("You are already on the latest version.");
    });
});
