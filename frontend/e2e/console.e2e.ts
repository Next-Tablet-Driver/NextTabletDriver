import { expect, test } from "./fixtures";

test.describe("Console screen", () => {
    test.beforeEach(async ({ app }) => {
        await app.open();
        await app.goTo("Console");
    });

    test("lists the log entries with their time, level, group and message", async ({ app }) => {
        await expect(app.content).toContainText("Sample info message number 1 from HID");
        await expect(app.content).toContainText("Sample error message number 4 from Plugins");
        await expect(app.content.getByRole("row")).not.toHaveCount(0);
    });

    test("identical consecutive lines are all listed", async ({ app }) => {
        await expect(app.content.getByRole("row").filter({ hasText: "Sample info message number 1 from HID" })).toHaveCount(2);
    });

    test("a level can be hidden with its toggle", async ({ app }) => {
        await expect(app.content).toContainText("Sample error message number 4");
        await app.content.getByRole("button", { name: "Error" }).click();
        await expect(app.content).not.toContainText("Sample error message number 4");
        await expect(app.content).toContainText("Sample info message number 1");
        await app.content.getByRole("button", { name: "Error" }).click();
        await expect(app.content).toContainText("Sample error message number 4");
    });

    test("searching narrows the list to the matching entries", async ({ app }) => {
        await app.content.getByPlaceholder("Search logs...").fill("tabletmanager");
        await expect(app.content).toContainText("from TabletManager");
        await expect(app.content).not.toContainText("from HID");
    });

    test("the search can be cleared", async ({ app }) => {
        await app.content.getByPlaceholder("Search logs...").fill("plugins");
        await expect(app.content).not.toContainText("from HID");
        await app.content.getByLabel("Clear search").click();
        await expect(app.content).toContainText("from HID");
    });

    test("searching for something absent leaves no entry", async ({ app }) => {
        await app.content.getByPlaceholder("Search logs...").fill("zzz-not-in-any-log");
        await expect(app.content).not.toContainText("Sample");
    });

    test("Clear Console asks the backend to clear the logs", async ({ app }) => {
        await app.content.getByRole("button", { name: "Clear Console" }).click();
        await expect.poll(async () => (await app.invocations("clear_logs")).length).toBe(1);
    });
});
