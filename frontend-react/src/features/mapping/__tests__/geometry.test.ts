import { describe, expect, it } from "vitest";
import {
  centreTarget,
  clampActive,
  clampRotation,
  desktopOf,
  dragActive,
  dragTarget,
  osuPlayfield,
  resizeActive,
  resizeTarget,
  snapAxis,
  targetCentre,
  type MonitorRect,
} from "@/features/mapping/geometry";

const left: MonitorRect = { x: 0, y: 0, width: 1920, height: 1080 };
const right: MonitorRect = { x: 1920, y: 0, width: 2560, height: 1440 };

describe("desktopOf", () => {
  it("spans every monitor", () => {
    expect(desktopOf([left, right])).toEqual({ minX: 0, minY: 0, width: 4480, height: 1440 });
  });

  it("handles a monitor left of the origin", () => {
    const desktop = desktopOf([{ x: -1920, y: 0, width: 1920, height: 1080 }, left]);
    expect(desktop).toMatchObject({ minX: -1920, width: 3840 });
  });

  it("falls back to 1920 x 1080 without monitors", () => {
    expect(desktopOf([])).toEqual({ minX: 0, minY: 0, width: 1920, height: 1080 });
  });
});

describe("snapAxis", () => {
  it("sticks the start to a near edge", () => {
    expect(snapAxis(50, 500, [0, 1920])).toBe(0);
  });

  it("sticks the end to a near edge", () => {
    expect(snapAxis(1400, 500, [0, 1920])).toBe(1420);
  });

  it("leaves an area that is far from every edge", () => {
    expect(snapAxis(600, 500, [0, 1920])).toBe(600);
  });
});

describe("the target area", () => {
  const desktop = desktopOf([left, right]);
  const target = { x: 100, y: 100, w: 800, h: 600 };

  it("is dragged and kept on the desktop", () => {
    const moved = dragTarget(target, 100000, -100000, desktop, [left, right], false);
    expect(moved).toEqual({ x: 4480 - 800, y: 0, w: 800, h: 600 });
  });

  it("snaps to a monitor edge while dragging when asked to", () => {
    const moved = dragTarget(target, -60, 0, desktop, [left, right], true);
    expect(moved.x).toBe(0);
    expect(dragTarget(target, -60, 0, desktop, [left, right], false).x).toBe(40);
  });

  it("is resized within the desktop and pulled back onto it", () => {
    const resized = resizeTarget({ ...target, x: 4000 }, { w: 1000 }, desktop);
    expect(resized).toMatchObject({ w: 1000, x: 3480 });
    expect(resizeTarget(target, { h: 99999 }, desktop).h).toBe(1440);
    expect(resizeTarget(target, { w: -5 }, desktop).w).toBe(1);
  });

  it("is placed by its centre", () => {
    const placed = centreTarget(target, { x: 1000, y: 500 }, desktop);
    expect(placed).toMatchObject({ x: 600, y: 200 });
    expect(targetCentre(placed, desktop)).toEqual({ x: 1000, y: 500 });
  });
});

describe("the active area", () => {
  const surface = { width: 160, height: 100 };
  const area = { x: 80, y: 50, w: 100, h: 50, rotation: 0 };

  it("stays on the surface when dragged", () => {
    expect(dragActive(area, 1000, -1000, surface)).toMatchObject({ x: 110, y: 25 });
  });

  it("is not clamped while the surface size is unknown", () => {
    expect(clampActive({ ...area, x: 999 }, { width: 0, height: 0 }).x).toBe(999);
  });

  it("keeps its proportions when the aspect ratio is locked", () => {
    const resized = resizeActive(area, { w: 60 }, surface, true);
    expect(resized).toMatchObject({ w: 60, h: 30 });
    const taller = resizeActive(area, { h: 40 }, surface, true);
    expect(taller).toMatchObject({ w: 80, h: 40 });
  });

  it("changes one side only when the aspect ratio is free", () => {
    expect(resizeActive(area, { w: 60 }, surface, false)).toMatchObject({ w: 60, h: 50 });
  });

  it("limits the other side to the surface when it follows", () => {
    expect(resizeActive(area, { w: 160 }, surface, true)).toMatchObject({ w: 160, h: 80 });
    const tall = { ...area, w: 50, h: 100 };
    expect(resizeActive(tall, { w: 160 }, surface, true)).toMatchObject({ w: 160, h: 100 });
  });

  it("rounds to two decimals", () => {
    expect(resizeActive(area, { w: 33.3333 }, surface, false).w).toBe(33.33);
  });

  it("limits the rotation to a turn each way", () => {
    expect(clampRotation(720)).toBe(360);
    expect(clampRotation(-12.3456)).toBe(-12.35);
  });
});

describe("osuPlayfield", () => {
  it("is centred horizontally on the active area", () => {
    const playfield = osuPlayfield({ x: 0, y: 0, w: 1920, h: 1080 });
    expect(playfield.left + playfield.width / 2).toBeCloseTo(50);
  });
});
