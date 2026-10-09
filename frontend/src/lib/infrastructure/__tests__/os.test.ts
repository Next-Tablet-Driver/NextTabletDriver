import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";

const shellOpen = vi.fn();
const dialogOpen = vi.fn();
const dialogSave = vi.fn();
const relaunch = vi.fn();
vi.mock("@tauri-apps/plugin-shell", () => ({ open: (...a: unknown[]) => shellOpen(...a) as unknown }));
vi.mock("@tauri-apps/plugin-dialog", () => ({
    open: (...a: unknown[]) => dialogOpen(...a) as unknown,
    save: (...a: unknown[]) => dialogSave(...a) as unknown,
}));
vi.mock("@tauri-apps/plugin-process", () => ({ relaunch: (...a: unknown[]) => relaunch(...a) as unknown }));

import { os } from "../os";

function setTauri(enabled: boolean): void {
    if (enabled) window.__TAURI_INTERNALS__ = {};
    else delete window.__TAURI_INTERNALS__;
}

describe("os abstraction layer", () => {
    beforeEach(() => {
        vi.clearAllMocks();
        vi.spyOn(console, "error").mockImplementation(() => undefined);
        vi.spyOn(console, "warn").mockImplementation(() => undefined);
    });
    afterEach(() => {
        setTauri(false);
        vi.restoreAllMocks();
    });

    it("opens URLs through the Tauri shell plugin inside Tauri", async () => {
        setTauri(true);
        await os.openUrl("https://github.com");
        expect(shellOpen).toHaveBeenCalledWith("https://github.com");
    });

    it("falls back to window.open in a plain browser", async () => {
        setTauri(false);
        const open = vi.spyOn(window, "open").mockReturnValue(null);
        await os.openUrl("https://github.com");
        expect(open).toHaveBeenCalledWith("https://github.com", "_blank");
        expect(shellOpen).not.toHaveBeenCalled();
    });

    it("swallows shell errors", async () => {
        setTauri(true);
        shellOpen.mockRejectedValueOnce(new Error("denied"));
        await expect(os.openUrl("https://x.y")).resolves.toBeUndefined();
    });

    it("returns the dialog selection inside Tauri and null outside", async () => {
        setTauri(true);
        dialogOpen.mockResolvedValueOnce("C:/a.json");
        await expect(os.openFileDialog()).resolves.toBe("C:/a.json");
        dialogSave.mockResolvedValueOnce("C:/b.json");
        await expect(os.saveFileDialog()).resolves.toBe("C:/b.json");

        setTauri(false);
        await expect(os.openFileDialog()).resolves.toBeNull();
        await expect(os.saveFileDialog()).resolves.toBeNull();
    });

    it("returns null when a dialog fails", async () => {
        setTauri(true);
        dialogOpen.mockRejectedValueOnce(new Error("nope"));
        await expect(os.openFileDialog()).resolves.toBeNull();
    });

    it("relaunches through the process plugin inside Tauri", async () => {
        setTauri(true);
        await os.relaunch();
        expect(relaunch).toHaveBeenCalledOnce();
    });
});
