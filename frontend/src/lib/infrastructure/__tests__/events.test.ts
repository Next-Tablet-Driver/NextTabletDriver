import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";

const listenMock = vi.fn();
vi.mock("@tauri-apps/api/event", () => ({ listen: (...a: unknown[]) => listenMock(...a) as unknown }));

import { events } from "../events";

describe("events abstraction layer", () => {
    beforeEach(() => {
        listenMock.mockReset();
        vi.spyOn(console, "error").mockImplementation(() => undefined);
    });
    afterEach(() => {
        delete window.__TAURI_INTERNALS__;
        vi.restoreAllMocks();
    });

    it("does not subscribe outside of Tauri", async () => {
        await expect(events.onTabletEvent(() => undefined)).resolves.toBeUndefined();
        await expect(events.onLogEvent(() => undefined)).resolves.toBeUndefined();
        expect(listenMock).not.toHaveBeenCalled();
    });

    it("unwraps the event payload for tablet events", async () => {
        window.__TAURI_INTERNALS__ = {};
        const unlisten = vi.fn();
        listenMock.mockResolvedValueOnce(unlisten);
        const callback = vi.fn();

        await expect(events.onTabletEvent(callback)).resolves.toBe(unlisten);
        expect(listenMock.mock.calls[0]?.[0]).toBe("tablet-event");

        const handler = listenMock.mock.calls[0]?.[1] as (e: { payload: unknown }) => void;
        handler({ payload: { x: 1, y: 2 } });
        expect(callback).toHaveBeenCalledWith({ x: 1, y: 2 });
    });

    it.each([
        ["onDeviceChanged", "device-changed"],
        ["onProfileLoaded", "profile-loaded"],
        ["onNavigateToUpdates", "navigate-to-updates"],
    ] as const)("%s listens to %s", async (method, eventName) => {
        window.__TAURI_INTERNALS__ = {};
        listenMock.mockResolvedValueOnce(vi.fn());
        const callback = vi.fn();
        await events[method](callback);
        expect(listenMock.mock.calls[0]?.[0]).toBe(eventName);
        (listenMock.mock.calls[0]?.[1] as () => void)();
        expect(callback).toHaveBeenCalledOnce();
    });

    it("returns undefined when listen fails", async () => {
        window.__TAURI_INTERNALS__ = {};
        listenMock.mockRejectedValueOnce(new Error("fail"));
        await expect(events.onLogEvent(() => undefined)).resolves.toBeUndefined();
    });
});
