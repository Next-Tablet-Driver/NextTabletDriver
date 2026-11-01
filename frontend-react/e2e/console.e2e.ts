import { expect, test } from "./fixtures";

test.describe("Console page", () => {
  test.beforeEach(async ({ app }) => {
    await app.open();
    await app.goTo("Console");
  });

  test("lists the log entries with their time, level, group and message", async ({ app }) => {
    await expect(app.content).toContainText("Sample info message number 1 from HID");
    await expect(app.content).toContainText("Sample error message number 4 from Plugins");
    await expect(app.content.getByRole("grid", { name: "Logs" })).toBeVisible();
    await expect(app.content.getByText("Group", { exact: true })).toBeVisible();
    await expect(app.content.getByRole("row")).not.toHaveCount(0);
  });

  test("identical consecutive lines are all listed", async ({ app }) => {
    await expect(
      app.content.getByRole("row").filter({ hasText: "Sample info message number 1 from HID" }),
    ).toHaveCount(2);
  });

  test("a level can be hidden with its toggle", async ({ app }) => {
    await expect(app.content).toContainText("Sample error message number 4");
    await app.content.getByRole("button", { name: "Error", exact: true }).click();
    await expect(app.content).not.toContainText("Sample error message number 4");
    await expect(app.content).toContainText("Sample info message number 1");
    await app.content.getByRole("button", { name: "Error", exact: true }).click();
    await expect(app.content).toContainText("Sample error message number 4");
  });

  test("searching narrows the list to the matching entries", async ({ app }) => {
    await app.field("Search the logs").fill("tabletmanager");
    await expect(app.content).toContainText("from TabletManager");
    await expect(app.content).not.toContainText("from HID");
  });

  test("searching for something absent says so", async ({ app }) => {
    await app.field("Search the logs").fill("zzz-not-in-any-log");
    await expect(app.content).not.toContainText("Sample");
    await expect(app.content).toContainText("No logs to display");
  });

  test("the count shows how many entries are visible", async ({ app }) => {
    await app.field("Search the logs").fill("plugins");
    await expect(app.content.getByText(/^\d+ \/ \d+$/)).toBeVisible();
  });

  test("Clear asks the backend to clear the logs", async ({ app }) => {
    await app.content.getByRole("button", { name: "Clear", exact: true }).click();
    await expect.poll(async () => (await app.invocations("clear_logs")).length).toBe(1);
  });
});
