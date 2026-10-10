import { defineConfig, devices } from "@playwright/test";

const PORT = 4173;
const isCI = process.env.CI !== undefined;

/**
 * End-to-end tests of the real Svelte app running in a browser against the Tauri IPC mock
 * (`src/dev/tauri-mock.ts`, enabled by `/?mock` on the dev server). They cover every screen
 * without a tablet or a native window; the native shell has its own smoke test in `e2e-native/`.
 */
export default defineConfig({
    testDir: "./e2e",
    testMatch: "**/*.e2e.ts",
    fullyParallel: true,
    forbidOnly: isCI,
    retries: isCI ? 1 : 0,
    workers: isCI ? 2 : undefined,
    reporter: isCI ? [["github"], ["html", { open: "never" }]] : [["list"]],
    use: {
        baseURL: `http://127.0.0.1:${String(PORT)}`,
        // Traces and screenshots are only kept for failures (uploaded as a CI artifact).
        trace: "retain-on-failure",
        screenshot: "only-on-failure",
        video: "retain-on-failure",
    },
    projects: [{ name: "chromium", use: { ...devices["Desktop Chrome"], viewport: { width: 1000, height: 900 } } }],
    webServer: {
        command: `npm run dev -- --host 127.0.0.1 --port ${String(PORT)} --strictPort`,
        url: `http://127.0.0.1:${String(PORT)}`,
        reuseExistingServer: !isCI,
        timeout: 120_000,
    },
});
