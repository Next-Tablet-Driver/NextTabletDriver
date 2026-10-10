/**
 * Smoke test of the real desktop app, driven through `tauri-driver` (W3C WebDriver).
 *
 * Unlike the Playwright suite (`e2e/`), nothing is mocked here: the Rust backend, the webview and
 * the IPC bridge are the ones that ship. It only checks that the app starts and that its main
 * screens render; the behaviour of each screen is covered by the Playwright suite.
 *
 *   node --experimental-strip-types e2e-native/smoke.ts [path/to/app]
 *
 * Requirements: `tauri-driver` in the PATH, plus `WebKitWebDriver` (Linux, under a display
 * server such as xvfb) or `msedgedriver` (Windows). The app reads and may write the settings of
 * the current user, so outside CI it only runs with NTD_SMOKE_LOCAL=1.
 */
import { spawn, spawnSync, type ChildProcess } from "node:child_process";
import { existsSync } from "node:fs";
import { resolve } from "node:path";

const DRIVER_URL = "http://127.0.0.1:4444";
const STARTUP_TIMEOUT_MS = 90_000;

const executable = process.platform === "win32" ? "app.exe" : "app";
const application = resolve(process.argv[2] ?? `../src-tauri/target/debug/${executable}`);

interface WebDriverResponse<T> {
    value: T;
}

async function webdriver<T>(method: "GET" | "POST" | "DELETE", path: string, body?: unknown): Promise<T> {
    const response = await fetch(`${DRIVER_URL}${path}`, {
        method,
        headers: { "content-type": "application/json" },
        body: body === undefined ? undefined : JSON.stringify(body),
    });
    const parsed = (await response.json()) as WebDriverResponse<T & { error?: string; message?: string }>;
    if (!response.ok) {
        throw new Error(`${method} ${path} failed (${String(response.status)}): ${parsed.value.error ?? ""} ${parsed.value.message ?? ""}`);
    }
    return parsed.value;
}

function log(line: string): void {
    process.stdout.write(`${line}
`);
}

function sleep(ms: number): Promise<void> {
    return new Promise((done) => setTimeout(done, ms));
}

async function waitFor<T>(what: string, probe: () => Promise<T | undefined>, timeoutMs = STARTUP_TIMEOUT_MS): Promise<T> {
    const deadline = Date.now() + timeoutMs;
    let lastError: unknown;
    while (Date.now() < deadline) {
        try {
            const value = await probe();
            if (value !== undefined) return value;
        } catch (error) {
            lastError = error;
        }
        await sleep(500);
    }
    throw new Error(`Timed out waiting for ${what}${lastError === undefined ? "" : ` (last error: ${lastError instanceof Error ? lastError.message : "unknown"})`}`);
}

class Session {
    readonly id: string;

    constructor(id: string) {
        this.id = id;
    }

    execute<T>(script: string, args: unknown[] = []): Promise<T> {
        return webdriver<T>("POST", `/session/${this.id}/execute/sync`, { script, args });
    }

    /** The visible text of the whole page. */
    text(): Promise<string> {
        return this.execute<string>("return document.body.innerText;");
    }

    /** Clicks the first button whose text is exactly `label`; false when there is none. */
    clickButton(label: string): Promise<boolean> {
        return this.execute<boolean>(
            `const button = [...document.querySelectorAll('button')].find((b) => b.textContent.trim() === arguments[0]);
             if (button) button.click();
             return Boolean(button);`,
            [label],
        );
    }

    async waitForText(expected: string): Promise<void> {
        await waitFor(`"${expected}" on screen`, async () => ((await this.text()).includes(expected) ? true : undefined), 20_000);
    }
}

function startDriver(): ChildProcess {
    const driver = spawn("tauri-driver", [], { stdio: ["ignore", "inherit", "inherit"] });
    driver.on("error", (error) => {
        console.error(`Could not start tauri-driver: ${error.message}`);
        process.exitCode = 1;
    });
    return driver;
}

/** Stops tauri-driver and what it started; the app may outlive its session (close-to-tray). */
function stopDriver(driver: ChildProcess): void {
    if (process.platform === "win32" && driver.pid !== undefined) {
        spawnSync("taskkill", ["/pid", String(driver.pid), "/T", "/F"], { stdio: "ignore" });
    } else {
        driver.kill();
    }
}

const failures: string[] = [];

async function check(name: string, run: () => Promise<void>): Promise<void> {
    try {
        await run();
        log(`  ok    ${name}`);
    } catch (error) {
        failures.push(name);
        log(`  FAIL  ${name}\n        ${error instanceof Error ? error.message : String(error)}`);
    }
}

function expectTruthy(condition: boolean, message: string): void {
    if (!condition) throw new Error(message);
}

async function main(): Promise<void> {
    if (process.env.CI === undefined && process.env.NTD_SMOKE_LOCAL !== "1") {
        console.error("This test starts the real app with the settings of the current user: set NTD_SMOKE_LOCAL=1 to run it outside CI.");
        process.exitCode = 2;
        return;
    }
    if (!existsSync(application)) {
        console.error(`The application was not found at ${application}: build it first (see .github/workflows/ci.yml).`);
        process.exitCode = 2;
        return;
    }

    const driver = startDriver();
    let session: Session | undefined;
    try {
        await waitFor("tauri-driver", async () => {
            await webdriver("GET", "/status");
            return true;
        }, 30_000);

        const created = await webdriver<{ sessionId: string }>("POST", "/session", {
            capabilities: { alwaysMatch: { "tauri:options": { application } } },
        });
        session = new Session(created.sessionId);
        log(`Driving ${application}`);

        await check("the main screen renders", async () => {
            await waitFor("the tab bar", async () => ((await session?.text())?.includes("Pen Settings") === true ? true : undefined));
        });

        // From here on, record anything the page reports as an error.
        await session.execute(
            `window.__smokeErrors = [];
             window.addEventListener('error', (e) => window.__smokeErrors.push('error: ' + e.message));
             window.addEventListener('unhandledrejection', (e) => window.__smokeErrors.push('rejection: ' + String(e.reason)));
             const original = console.error.bind(console);
             console.error = (...args) => {
                 const line = args.map(String).join(' ');
                 // The startup update check cannot succeed on a build agent or before a release exists.
                 if (!line.startsWith('Failed to check for updates')) window.__smokeErrors.push('console.error: ' + line);
                 original(...args);
             };`,
        );

        await check("the backend answers: the title bar shows the core version", async () => {
            const text = await session?.text();
            expectTruthy(/NextTabletDriver v\d+\.\d+\.\d+/.test(text ?? ""), `no version in the title bar: ${text?.slice(0, 200) ?? ""}`);
        });

        await check("the footer shows the profile", async () => {
            await session?.waitForText("Profile:");
        });

        await check("the window has a size", async () => {
            const rect = await webdriver<{ width: number; height: number }>("GET", `/session/${session?.id ?? ""}/window/rect`);
            expectTruthy(rect.width > 0 && rect.height > 0, `empty window: ${JSON.stringify(rect)}`);
        });

        const screens: [string, string][] = [
            ["Filters", "AVAILABLE FILTERS"],
            ["Pen Settings", "Hardware Toggles"],
            ["Console", "Clear Console"],
            ["Settings", "General Settings"],
            ["Output", "Display"],
        ];
        for (const [tab, marker] of screens) {
            await check(`the ${tab} screen opens`, async () => {
                expectTruthy((await session?.clickButton(tab)) === true, `no "${tab}" tab`);
                await session?.waitForText(marker);
                // Give the screen a moment to finish its own loading, then see what it reported.
                await sleep(500);
                const errors = await session?.execute<string[]>("return window.__smokeErrors.splice(0);");
                expectTruthy(errors?.length === 0, `the ${tab} screen reported: ${JSON.stringify(errors)}`);
            });
        }

        await check("no error was reported by the page", async () => {
            const errors = await session?.execute<string[]>("return window.__smokeErrors;");
            expectTruthy(errors?.length === 0, `the page reported: ${JSON.stringify(errors)}`);
        });
    } catch (error) {
        failures.push("setup");
        console.error(error instanceof Error ? error.message : String(error));
    } finally {
        if (session !== undefined) await webdriver("DELETE", `/session/${session.id}`).catch(() => undefined);
        stopDriver(driver);
    }

    if (failures.length > 0) {
        console.error(`\n${String(failures.length)} check(s) failed: ${failures.join(", ")}`);
        process.exitCode = 1;
    } else {
        log("\nThe real app starts and renders every main screen.");
    }
}

await main();
