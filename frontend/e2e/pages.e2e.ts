import { expect, test } from "./fixtures";

test.describe("Release screen", () => {
    test.beforeEach(async ({ app }) => {
        await app.open();
        await app.goTo("Release");
    });

    test("lists the releases, newest first, and marks the latest one", async ({ app }) => {
        await expect(app.content).toContainText("Release Notes");
        const text = await app.content.innerText();
        expect(text.indexOf("v2.0.0")).toBeGreaterThanOrEqual(0);
        expect(text.indexOf("v2.0.0")).toBeLessThan(text.indexOf("v1.9.0"));
        await expect(app.content).toContainText(/latest/i);
    });

    test("shows the changes of a release without the markdown noise", async ({ app }) => {
        await expect(app.content).toContainText("New Tauri interface by @iSweat-exe");
        await expect(app.content).toContainText("Add WebSocket overlay support by @iSweat-exe");
        await expect(app.content).not.toContainText("https://github.com/Next-Tablet-Driver/NextTabletDriver/pull/101");
    });

    test("shows the publication dates", async ({ app }) => {
        await expect(app.content).toContainText("October 1, 2026");
        await expect(app.content).toContainText("August 12, 2026");
    });
});

test.describe("Credits screen", () => {
    test.beforeEach(async ({ app }) => {
        await app.open();
        await app.goTo("Credits");
    });

    test("shows the download, star and release counters", async ({ app }) => {
        await expect(app.content).toContainText("1,520");
        await expect(app.content).toContainText(/downloads/i);
        await expect(app.content).toContainText("321");
        await expect(app.content).toContainText(/stars/i);
    });

    test("lists the contributors with the maintainer first", async ({ app }) => {
        const text = await app.content.innerText();
        expect(text.indexOf("@iSweat-exe")).toBeLessThan(text.indexOf("@contributor-one"));
        expect(text.indexOf("@contributor-one")).toBeLessThan(text.indexOf("@contributor-two"));
        await expect(app.content).toContainText(/maintainer/i);
        await expect(app.content).toContainText("412 commits");
    });
});
