import { finite, type ActiveArea, type Rect } from "@/features/mapping/geometry";
import type { Config } from "@/infrastructure/tauri/commands";

/** The target area of a configuration, with the `null` numbers of the backend read as 0. */
export function targetOf(config: Config): Rect {
  const { x, y, w, h } = config.target_area;
  return { x: finite(x), y: finite(y), w: finite(w), h: finite(h) };
}

/** The active area of a configuration. `x` and `y` are its centre, in millimetres. */
export function activeOf(config: Config): ActiveArea {
  const { x, y, w, h, rotation } = config.active_area;
  return { x: finite(x), y: finite(y), w: finite(w), h: finite(h), rotation: finite(rotation) };
}

export function withTarget(config: Config, target: Rect): Config {
  return { ...config, target_area: { x: target.x, y: target.y, w: target.w, h: target.h } };
}

export function withActive(config: Config, area: ActiveArea): Config {
  return {
    ...config,
    active_area: { x: area.x, y: area.y, w: area.w, h: area.h, rotation: area.rotation },
  };
}
