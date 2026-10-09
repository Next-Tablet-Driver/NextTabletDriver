import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import type { Config } from "../../../infrastructure/tauri/commands";

const getConfig = vi.fn();
const setConfig = vi.fn();
const saveConfig = vi.fn();
vi.mock("../../../infrastructure/tauri/commands", () => ({
    getConfig: () => getConfig() as unknown,
    setConfig: (c: unknown) => setConfig(c) as unknown,
    saveConfig: () => saveConfig() as unknown,
}));

import { MappingState } from "../mapping-state.svelte";

function makeConfig(): Config {
    return {
        target_area: { x: 0, y: 0, w: 1920, h: 1080 },
        active_area: { x: 10, y: 20, w: 100, h: 60, rotation: 0 },
        display_snapping: true,
        lock_aspect_ratio: true,
        show_osu_playfield: true,
        mode: "Absolute",
    } as Config;
}

const DEBOUNCE = 1500;

async function loaded(): Promise<MappingState> {
    const state = new MappingState();
    await state.load();
    await vi.advanceTimersByTimeAsync(60); // `isLoaded` flips after 50 ms
    return state;
}

describe("MappingState", () => {
    beforeEach(() => {
        vi.useFakeTimers();
        getConfig.mockReset().mockResolvedValue(makeConfig());
        setConfig.mockReset().mockResolvedValue(undefined);
        saveConfig.mockReset().mockResolvedValue(undefined);
        vi.spyOn(console, "error").mockImplementation(() => undefined);
    });
    afterEach(() => {
        vi.useRealTimers();
        vi.restoreAllMocks();
    });

    it("loads the backend config into bindable state", async () => {
        const state = await loaded();
        expect(state.isLoaded).toBe(true);
        expect(state.isDirty).toBe(false);
        expect(state.tabletX).toBe(10);
        expect(state.tabletW).toBe(100);
        expect(state.targetW).toBe(1920);
        expect(state.driverMode).toBe("Absolute");
    });

    it("stays unloaded and does not throw when the backend fails", async () => {
        getConfig.mockRejectedValueOnce(new Error("backend down"));
        const state = new MappingState();
        await state.load();
        expect(state.isLoaded).toBe(false);
        expect(state.config).toBeNull();
    });

    it("debounces markDirty into a single setConfig and flags the state dirty", async () => {
        const state = await loaded();
        state.tabletW = 120;
        state.markDirty();
        state.tabletW = 130;
        state.markDirty();

        await vi.advanceTimersByTimeAsync(DEBOUNCE - 1);
        expect(setConfig).not.toHaveBeenCalled();

        await vi.advanceTimersByTimeAsync(2);
        expect(setConfig).toHaveBeenCalledTimes(1);
        const sent = setConfig.mock.calls[0]?.[0] as Config;
        expect(sent.active_area.w).toBe(130);
        expect(state.isDirty).toBe(true);
    });

    it("ignores markDirty before the config is loaded", () => {
        const state = new MappingState();
        state.markDirty();
        vi.advanceTimersByTime(DEBOUNCE * 2);
        expect(setConfig).not.toHaveBeenCalled();
    });

    it("undoes, redoes and reverts to the initial configuration", async () => {
        const state = await loaded();

        state.tabletW = 150;
        state.markDirty();
        await vi.advanceTimersByTimeAsync(DEBOUNCE + 1);
        state.tabletW = 200;
        state.markDirty();
        await vi.advanceTimersByTimeAsync(DEBOUNCE + 1);
        expect(state.tabletW).toBe(200);

        state.undo();
        expect(state.tabletW).toBe(150);
        state.redo();
        expect(state.tabletW).toBe(200);

        state.revertToInitial();
        expect(state.tabletW).toBe(100);
        expect(state.isDirty).toBe(false);
    });

    it("undo is a no-op at the start of history", async () => {
        const state = await loaded();
        state.undo();
        expect(state.tabletW).toBe(100);
        expect(setConfig).not.toHaveBeenCalled();
    });

    it("save persists only when dirty and then clears the flag", async () => {
        const state = await loaded();
        await state.save();
        expect(saveConfig).not.toHaveBeenCalled();

        state.tabletH = 80;
        state.markDirty();
        await vi.advanceTimersByTimeAsync(DEBOUNCE + 1);
        expect(state.isDirty).toBe(true);

        await state.save();
        expect(saveConfig).toHaveBeenCalledOnce();
        expect(state.isDirty).toBe(false);
    });
});
