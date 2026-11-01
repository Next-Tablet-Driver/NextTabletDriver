import type { CSSProperties, ReactNode, Ref } from "react";

const STAGE_HEIGHT_PX = 260;
const RULER_PX = 18;

/** A tick spacing of 1, 2 or 5 times a power of ten that gives about `target` ticks. */
export function niceStep(extent: number, target = 10): number {
  const raw = Math.max(extent, 1) / target;
  const power = 10 ** Math.floor(Math.log10(raw));
  const unit = [1, 2, 5, 10].find((candidate) => candidate * power >= raw) ?? 10;
  return unit * power;
}

/** The values at which a ruler has a tick, from `origin` to `origin + extent`. */
export function ticks(origin: number, extent: number, step: number): number[] {
  const first = Math.ceil(origin / step) * step;
  const values: number[] = [];
  for (let value = first; value <= origin + extent; value += step) {
    values.push(value);
  }
  return values;
}

/**
 * The drawing surface of an area editor: a blue print of the thing it shows (the desktop, the
 * tablet) with a millimetre or pixel grid and a ruler on two edges, as large as the page allows.
 * Children are placed in percentages of the surface. It is a picture of numbers that are also
 * editable as text, so the rulers are decoration for assistive technology.
 */
export function AreaStage({
  width,
  height,
  originX = 0,
  originY = 0,
  unit,
  label,
  stageRef,
  children,
}: {
  /** Extent of what is drawn, in `unit`. */
  width: number;
  height: number;
  /** The value at the top left corner, for a desktop that starts left of the main screen. */
  originX?: number;
  originY?: number;
  unit: "px" | "mm";
  label: string;
  stageRef?: Ref<HTMLDivElement>;
  children: ReactNode;
}) {
  const ratio = width / height;
  const stepX = niceStep(width);
  const stepY = niceStep(height, 7);
  const surface: CSSProperties = {
    aspectRatio: String(ratio),
    width: `min(100%, ${String(Math.round(STAGE_HEIGHT_PX * ratio))}px)`,
    backgroundColor: "var(--stage-paper)",
    backgroundImage: [
      "linear-gradient(to right, var(--stage-line) 1px, transparent 1px)",
      "linear-gradient(to bottom, var(--stage-line) 1px, transparent 1px)",
      "linear-gradient(to right, var(--stage-line-major) 1px, transparent 1px)",
      "linear-gradient(to bottom, var(--stage-line-major) 1px, transparent 1px)",
    ].join(", "),
    backgroundSize: [
      `${String((stepX / width) * 100)}% 100%`,
      `100% ${String((stepY / height) * 100)}%`,
      `${String(((stepX * 5) / width) * 100)}% 100%`,
      `100% ${String(((stepY * 5) / height) * 100)}%`,
    ].join(", "),
    backgroundPosition: [
      `${String(((Math.ceil(originX / stepX) * stepX - originX) / width) * 100)}% 0`,
      `0 ${String(((Math.ceil(originY / stepY) * stepY - originY) / height) * 100)}%`,
      `${String(((Math.ceil(originX / (stepX * 5)) * stepX * 5 - originX) / width) * 100)}% 0`,
      `0 ${String(((Math.ceil(originY / (stepY * 5)) * stepY * 5 - originY) / height) * 100)}%`,
    ].join(", "),
  };

  return (
    <div
      className="flex justify-center py-3 pl-8"
      style={{ minHeight: STAGE_HEIGHT_PX + 2 * RULER_PX }}
    >
      <div className="flex flex-col items-start" style={{ width: surface.width }}>
        <div className="relative w-full" style={{ height: RULER_PX }} aria-hidden="true">
          {ticks(originX, width, stepX).map((value) => (
            <span
              key={value}
              className="absolute bottom-0 -translate-x-1/2 text-[11px] leading-none text-tertiary tabular-nums"
              style={{ left: `${String(((value - originX) / width) * 100)}%` }}
            >
              {value}
            </span>
          ))}
        </div>
        <div className="flex w-full">
          <div className="relative w-0" aria-hidden="true">
            {ticks(originY, height, stepY).map((value) => (
              <span
                key={value}
                className="absolute right-1.5 -translate-y-1/2 text-[11px] leading-none text-tertiary tabular-nums"
                style={{ top: `${String(((value - originY) / height) * 100)}%` }}
              >
                {value}
              </span>
            ))}
          </div>
          <div
            ref={stageRef}
            role="group"
            aria-label={`${label}, in ${unit}`}
            className="relative w-full overflow-hidden rounded-xs"
            style={surface}
          >
            {children}
          </div>
        </div>
      </div>
    </div>
  );
}
