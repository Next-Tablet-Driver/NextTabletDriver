import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";

const win = {
    minimize: vi.fn(),
    hide: vi.fn(),
    toggleMaximize: vi.fn(),
    close: vi.fn(),
};
const availableMonitors = vi.fn();

vi.mock("@tauri-apps/api/window", () => {
    class PhysicalSize {
        constructor(
            public width: number,
            public height: number,
        ) {}
    }
    class PhysicalPosition {
        constructor(
            public x: number,
            public y: number,
        ) {}
    }
    return {
        getCurrentWindow: () => win,
        availableMonitors: () => availableMonitors() as unknown,
        PhysicalSize,
        PhysicalPosition,
    };
});

import { windowControls } from "../window";

function setTauri(enabled: boolean): void {
    if (enabled) window.__TAURI_INTERNALS__ = {};
    else delete window.__TAURI_INTERNALS__;
}

const actions = [
    ["minimize", () => win.minimize],
    ["hide", () => win.hide],
    ["toggleMaximize", () => win.toggleMaximize],
    ["close", () => win.close],
] as const;

describe("windowControls", () => {
    beforeEach(() => {
        vi.clearAllMocks();
        for (const fn of Object.values(win)) fn.mockResolvedValue(undefined);
        vi.spyOn(console, "error").mockImplementation(() => undefined);
    });
    afterEach(() => {
        setTauri(false);
        vi.restoreAllMocks();
    });

    describe.each(actions)("%s", (name, target) => {
        it("drives the Tauri window when running inside the app", async () => {
            setTauri(true);
            await windowControls[name]();
            expect(target()).toHaveBeenCalledOnce();
        });

        it("logs instead of throwing when the window call fails", async () => {
            setTauri(true);
            target().mockRejectedValue(new Error("denied"));
            await expect(windowControls[name]()).resolves.toBeUndefined();
            expect(console.error).toHaveBeenCalled();
        });
    });

    it("does nothing in a plain browser, except closing the tab", async () => {
        const close = vi.spyOn(window, "close").mockImplementation(() => undefined);
        await windowControls.minimize();
        await windowControls.hide();
        await windowControls.toggleMaximize();
        for (const [, target] of actions) expect(target()).not.toHaveBeenCalled();
        expect(close).not.toHaveBeenCalled();

        await windowControls.close();
        expect(close).toHaveBeenCalledOnce();
        expect(win.close).not.toHaveBeenCalled();
    });

    describe("getMonitors", () => {
        it("lists the monitors known to Tauri", async () => {
            setTauri(true);
            availableMonitors.mockResolvedValue([{ name: "DELL" }]);
            await expect(windowControls.getMonitors()).resolves.toEqual([{ name: "DELL" }]);
        });

        it("returns one mock monitor in a plain browser", async () => {
            const monitors = await windowControls.getMonitors();
            expect(monitors).toHaveLength(1);
            expect(monitors[0]?.name).toBe("Mock Monitor");
            expect(monitors[0]?.size).toMatchObject({ width: 1920, height: 1080 });
            expect(availableMonitors).not.toHaveBeenCalled();
        });

        it("returns no monitor when the query fails", async () => {
            setTauri(true);
            availableMonitors.mockRejectedValue(new Error("nope"));
            await expect(windowControls.getMonitors()).resolves.toEqual([]);
            expect(console.error).toHaveBeenCalled();
        });
    });
});
