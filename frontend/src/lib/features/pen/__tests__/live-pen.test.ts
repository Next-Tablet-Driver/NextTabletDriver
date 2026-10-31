import { describe, it, expect } from "vitest";
import { LivePen } from "../live-pen.svelte";

const event = (status: string, pressure: number, max?: number): Parameters<LivePen["update"]>[0] => ({
    status, x: 0, y: 0, pressure, max_pressure: max, hover_distance: 0, tilt_x: 0, tilt_y: 0,
});

describe("LivePen", () => {
    it("starts with no pen", () => {
        const live = new LivePen();
        expect(live.present).toBe(false);
        expect(live.touching).toBe(false);
        expect(live.ratio).toBe(0);
    });

    it("turns the raw pressure into a share of the maximum", () => {
        const live = new LivePen();
        live.update(event("Contact", 2048, 8192));
        expect(live.ratio).toBeCloseTo(0.25);
        expect(live.touching).toBe(true);
    });

    it("follows a pen that hovers without touching", () => {
        const live = new LivePen();
        live.update(event("Hover", 0, 8192));
        expect(live.present).toBe(true);
        expect(live.touching).toBe(false);
    });

    it("loses the pen when it leaves the tablet", () => {
        const live = new LivePen();
        live.update(event("Contact", 100, 1000));
        live.update(event("OutOfRange", 0, 1000));
        expect(live.present).toBe(false);
        expect(live.touching).toBe(false);
    });

    it("keeps the share between 0 and 1", () => {
        const live = new LivePen();
        live.update(event("Contact", 5000, 1000));
        expect(live.ratio).toBe(1);
    });

    it("shows no pressure while the maximum is unknown", () => {
        const live = new LivePen();
        live.update(event("Contact", 500));
        expect(live.ratio).toBe(0);
        live.update(event("Contact", 500, 0));
        expect(live.ratio).toBe(0);
    });

    it("forgets the pen when it is stopped", () => {
        const live = new LivePen();
        live.update(event("Contact", 500, 1000));
        live.stop();
        expect(live.present).toBe(false);
        expect(live.ratio).toBe(0);
    });
});
