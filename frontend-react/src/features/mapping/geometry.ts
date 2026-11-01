/**
 * The arithmetic behind the two area editors: where the target area may sit on the desktop, where
 * the active area may sit on the tablet, and how they snap and keep their proportions.
 * Plain functions, so every rule can be tested without a screen.
 */

/** The backend sends `null` for a number it could not represent; a valid profile has none. */
export const finite = (value: number | null | undefined): number => value ?? 0;

export const clamp = (value: number, min: number, max: number): number =>
  Math.max(min, Math.min(value, max));

/** Rounds to two decimals, which is what a millimetre value is shown with. */
export const round2 = (value: number): number => Math.round(value * 100) / 100;

export interface Rect {
  x: number;
  y: number;
  w: number;
  h: number;
}

export interface MonitorRect {
  x: number;
  y: number;
  width: number;
  height: number;
}

/** The smallest rectangle that holds every monitor (the virtual desktop). */
export interface Desktop {
  minX: number;
  minY: number;
  width: number;
  height: number;
}

const FALLBACK_DESKTOP: Desktop = { minX: 0, minY: 0, width: 1920, height: 1080 };

export function desktopOf(monitors: readonly MonitorRect[]): Desktop {
  if (monitors.length === 0) {
    return FALLBACK_DESKTOP;
  }
  const minX = Math.min(...monitors.map((m) => m.x));
  const minY = Math.min(...monitors.map((m) => m.y));
  const maxX = Math.max(...monitors.map((m) => m.x + m.width));
  const maxY = Math.max(...monitors.map((m) => m.y + m.height));
  return { minX, minY, width: maxX - minX || 1920, height: maxY - minY || 1080 };
}

/** Distance, in pixels, within which a moved area sticks to a monitor edge. */
export const SNAP_DISTANCE_PX = 120;

/** Moves `start` to the closest monitor edge when its own edge is near one. */
export function snapAxis(
  start: number,
  size: number,
  edges: readonly number[],
  distance = SNAP_DISTANCE_PX,
): number {
  let best = start;
  let bestDistance = Infinity;
  for (const edge of edges) {
    const fromStart = Math.abs(start - edge);
    if (fromStart < distance && fromStart < bestDistance) {
      bestDistance = fromStart;
      best = edge;
    }
    const fromEnd = Math.abs(start + size - edge);
    if (fromEnd < distance && fromEnd < bestDistance) {
      bestDistance = fromEnd;
      best = edge - size;
    }
  }
  return best;
}

export function monitorEdges(monitors: readonly MonitorRect[], axis: "x" | "y"): number[] {
  return monitors.flatMap((m) => (axis === "x" ? [m.x, m.x + m.width] : [m.y, m.y + m.height]));
}

/** Keeps the target area inside the desktop. */
export function clampTarget(target: Rect, desktop: Desktop): Rect {
  return {
    ...target,
    x: Math.round(clamp(target.x, desktop.minX, desktop.minX + desktop.width - target.w)),
    y: Math.round(clamp(target.y, desktop.minY, desktop.minY + desktop.height - target.h)),
  };
}

/** Moves the target area by `dx`, `dy` desktop pixels from where a drag started. */
export function dragTarget(
  start: Rect,
  dx: number,
  dy: number,
  desktop: Desktop,
  monitors: readonly MonitorRect[],
  snapping: boolean,
): Rect {
  let x = start.x + dx;
  let y = start.y + dy;
  if (snapping) {
    x = snapAxis(x, start.w, monitorEdges(monitors, "x"));
    y = snapAxis(y, start.h, monitorEdges(monitors, "y"));
  }
  return clampTarget({ ...start, x, y }, desktop);
}

/** The target size, limited to the desktop, with the area kept on it. */
export function resizeTarget(
  target: Rect,
  change: { w?: number; h?: number },
  desktop: Desktop,
): Rect {
  const w = change.w === undefined ? target.w : Math.round(clamp(change.w, 1, desktop.width));
  const h = change.h === undefined ? target.h : Math.round(clamp(change.h, 1, desktop.height));
  return clampTarget({ ...target, w, h }, desktop);
}

/** The target centre, which the interface shows as X and Y, relative to the desktop. */
export function targetCentre(target: Rect, desktop: Desktop): { x: number; y: number } {
  return {
    x: Math.round(target.x - desktop.minX + target.w / 2),
    y: Math.round(target.y - desktop.minY + target.h / 2),
  };
}

/** Puts the target area so that its centre is at `centre` (relative to the desktop). */
export function centreTarget(
  target: Rect,
  centre: { x?: number; y?: number },
  desktop: Desktop,
): Rect {
  const x = centre.x === undefined ? target.x : centre.x - target.w / 2 + desktop.minX;
  const y = centre.y === undefined ? target.y : centre.y - target.h / 2 + desktop.minY;
  return clampTarget({ ...target, x, y }, desktop);
}

/** The tablet surface, in millimetres; `0` while the size is unknown. */
export interface Surface {
  width: number;
  height: number;
}

/** The active area: `x` and `y` are its centre. */
export interface ActiveArea extends Rect {
  rotation: number;
}

/** Keeps the active area on the tablet surface (when its size is known). */
export function clampActive(area: ActiveArea, surface: Surface): ActiveArea {
  const x = surface.width > 0 ? clamp(area.x, area.w / 2, surface.width - area.w / 2) : area.x;
  const y = surface.height > 0 ? clamp(area.y, area.h / 2, surface.height - area.h / 2) : area.y;
  return { ...area, x: round2(x), y: round2(y) };
}

/** Moves the active area by `dx`, `dy` millimetres from where a drag started. */
export function dragActive(
  start: ActiveArea,
  dx: number,
  dy: number,
  surface: Surface,
): ActiveArea {
  return clampActive({ ...start, x: start.x + dx, y: start.y + dy }, surface);
}

/**
 * Sets the width or the height of the active area. With the aspect ratio locked, the other side
 * follows so that the proportions stay the same.
 */
export function resizeActive(
  area: ActiveArea,
  change: { w: number } | { h: number },
  surface: Surface,
  lockAspect: boolean,
): ActiveArea {
  const limit = (value: number, max: number): number =>
    max > 0 ? clamp(value, 1, max) : Math.max(1, value);
  const ratio = area.h > 0 ? area.w / area.h : 0;
  let { w, h } = area;
  if ("w" in change) {
    w = limit(change.w, surface.width);
    if (lockAspect && ratio > 0) {
      h = limit(w / ratio, surface.height);
    }
  } else {
    h = limit(change.h, surface.height);
    if (lockAspect && ratio > 0) {
      w = limit(h * ratio, surface.width);
    }
  }
  return clampActive({ ...area, w: round2(w), h: round2(h) }, surface);
}

/** Sets the centre of the active area, keeping it on the surface. */
export function moveActive(
  area: ActiveArea,
  centre: { x?: number; y?: number },
  surface: Surface,
): ActiveArea {
  return clampActive({ ...area, x: centre.x ?? area.x, y: centre.y ?? area.y }, surface);
}

export const clampRotation = (degrees: number): number => round2(clamp(degrees, -360, 360));

/** osu!'s playfield, as a share of the target area: the guide drawn on the active area. */
export const OSU_PLAYFIELD_ASPECT = 1316 / 1080;
export const OSU_MENU_INNER_H = 1028;
export const OSU_MENU_Y_OFFSET = 496;

export function osuPlayfield(target: Rect): {
  left: number;
  top: number;
  width: number;
  height: number;
} {
  const width = ((target.h * OSU_PLAYFIELD_ASPECT) / target.w) * 100;
  return {
    width,
    height: (OSU_MENU_INNER_H / target.h) * 100,
    left: 50 - width / 2,
    top: 50 - (OSU_MENU_Y_OFFSET / target.h) * 100,
  };
}
