import { describe, it, expect, vi, beforeEach } from "vitest";

const invokeMock = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({ invoke: (...args: unknown[]) => invokeMock(...args) as unknown }));

import * as commands from "../tauri/commands";

describe("tauri commands", () => {
    beforeEach(() => {
        invokeMock.mockReset();
        invokeMock.mockResolvedValue(undefined);
    });

    it.each([
        ["getCoreVersion", "get_core_version"],
        ["getTabletStatus", "get_tablet_status"],
        ["getConfig", "get_config"],
        ["saveConfig", "save_config"],
        ["getMetrics", "get_metrics"],
        ["resetConfig", "reset_config"],
        ["getPresets", "get_presets"],
        ["getCurrentProfileName", "get_current_profile_name"],
        ["getAvailablePlugins", "get_available_plugins"],
        ["reloadPlugins", "reload_plugins"],
        ["openPluginsFolder", "open_plugins_folder"],
        ["openThemesFolder", "open_themes_folder"],
    ] as const)("%s invokes %s without arguments", async (fn, command) => {
        await (commands[fn] as () => Promise<unknown>)();
        expect(invokeMock).toHaveBeenCalledWith(command);
    });

    it.each([
        ["loadProfile", "load_profile"],
        ["importOtdProfile", "import_otd_profile"],
        ["exportProfile", "export_profile"],
    ] as const)("%s forwards the path to %s", async (fn, command) => {
        await commands[fn]("C:/profiles/a.json");
        expect(invokeMock).toHaveBeenCalledWith(command, { path: "C:/profiles/a.json" });
    });

    it("deletePlugin forwards the plugin id", async () => {
        await commands.deletePlugin("kalman");
        expect(invokeMock).toHaveBeenCalledWith("delete_plugin", { id: "kalman" });
    });

    // The backend owns the file picker: the webview must never send a filesystem path.
    it("installPlugin sends no path and reports whether a plugin was installed", async () => {
        invokeMock.mockResolvedValueOnce(true);
        await expect(commands.installPlugin()).resolves.toBe(true);
        expect(invokeMock).toHaveBeenCalledWith("install_plugin");
        expect(invokeMock.mock.calls[0]).toHaveLength(1);

        invokeMock.mockResolvedValueOnce(false);
        await expect(commands.installPlugin()).resolves.toBe(false);
    });

    it("log commands are no-ops outside of Tauri and call the backend inside it", async () => {
        await expect(commands.getLogs()).resolves.toEqual([]);
        await commands.clearLogs();
        expect(invokeMock).not.toHaveBeenCalled();

        window.__TAURI_INTERNALS__ = {};
        try {
            await commands.getLogs();
            await commands.clearLogs();
            expect(invokeMock).toHaveBeenCalledWith("get_logs");
            expect(invokeMock).toHaveBeenCalledWith("clear_logs");
        } finally {
            delete window.__TAURI_INTERNALS__;
        }
    });

    it("propagates backend errors", async () => {
        invokeMock.mockRejectedValueOnce("boom");
        await expect(commands.getConfig()).rejects.toBe("boom");
    });
});
