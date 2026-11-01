import { useEffect, useMemo, useRef, useState, type KeyboardEvent, type PointerEvent } from "react";
import { Toggle } from "@/components/base/toggle/toggle";
import { InputNumber } from "@/components/base/input/input-number";
import { AreaStage } from "@/features/mapping/AreaStage";
import { targetOf, withTarget } from "@/features/mapping/config-areas";
import {
  centreTarget,
  desktopOf,
  dragTarget,
  resizeTarget,
  targetCentre,
  clampTarget,
  type MonitorRect,
  type Rect,
} from "@/features/mapping/geometry";
import { useMappingSnapshot, useMappingStore } from "@/features/mapping/MappingProvider";
import { log } from "@/infrastructure/log";
import { windowControls } from "@/infrastructure/tauri/window";
import { Row, Section } from "@/ui/Page";

const KEY_STEP_PX = 10;
const ARROWS: Record<string, [number, number]> = {
  ArrowLeft: [-1, 0],
  ArrowRight: [1, 0],
  ArrowUp: [0, -1],
  ArrowDown: [0, 1],
};

/** The screens of the computer, and the part of them the tablet is mapped to. */
export function DisplayEditor() {
  const { config } = useMappingSnapshot();
  const mapping = useMappingStore();
  const [monitors, setMonitors] = useState<MonitorRect[]>([]);
  const stage = useRef<HTMLDivElement>(null);
  const drag = useRef<{ x: number; y: number; start: Rect } | null>(null);

  useEffect(() => {
    windowControls.getMonitors().then(
      (list) => {
        setMonitors(
          list.map((m) => ({
            x: m.position.x,
            y: m.position.y,
            width: m.size.width,
            height: m.size.height,
          })),
        );
      },
      (e: unknown) => {
        log.warn("Could not load display data", e);
      },
    );
  }, []);

  const desktop = useMemo(() => desktopOf(monitors), [monitors]);
  const target = config === null ? null : targetOf(config);

  // A profile that was never set up maps 1920 x 1080 at the origin: start at the desktop centre.
  const isDefault =
    target !== null && target.w === 1920 && target.h === 1080 && target.x === 0 && target.y === 0;
  const hasMonitors = monitors.length > 0;
  useEffect(() => {
    if (hasMonitors && isDefault) {
      mapping.update((c) =>
        withTarget(
          c,
          centreTarget(targetOf(c), { x: desktop.width / 2, y: desktop.height / 2 }, desktop),
        ),
      );
    }
  }, [hasMonitors, isDefault, desktop, mapping]);

  if (config === null || target === null) {
    return null;
  }

  const setTarget = (next: Rect): void => {
    mapping.update((c) => withTarget(c, next));
  };
  const snapping = config.display_snapping;
  const centre = targetCentre(target, desktop);

  const onPointerDown = (e: PointerEvent<HTMLDivElement>): void => {
    drag.current = { x: e.clientX, y: e.clientY, start: target };
    e.currentTarget.setPointerCapture(e.pointerId);
  };
  const onPointerMove = (e: PointerEvent<HTMLDivElement>): void => {
    const started = drag.current;
    const box = stage.current?.getBoundingClientRect();
    if (started === null || box === undefined || box.width === 0) {
      return;
    }
    const scale = desktop.width / box.width;
    setTarget(
      dragTarget(
        started.start,
        (e.clientX - started.x) * scale,
        (e.clientY - started.y) * scale,
        desktop,
        monitors,
        snapping,
      ),
    );
  };
  const onPointerUp = (e: PointerEvent<HTMLDivElement>): void => {
    drag.current = null;
    if (e.currentTarget.hasPointerCapture(e.pointerId)) {
      e.currentTarget.releasePointerCapture(e.pointerId);
    }
  };
  const onKeyDown = (e: KeyboardEvent<HTMLDivElement>): void => {
    const arrow = ARROWS[e.key];
    if (arrow === undefined) {
      return;
    }
    e.preventDefault();
    const step = KEY_STEP_PX * (e.shiftKey ? 10 : 1);
    setTarget(
      clampTarget(
        { ...target, x: target.x + arrow[0] * step, y: target.y + arrow[1] * step },
        desktop,
      ),
    );
  };

  const percent = (value: number, total: number): string => `${String((value / total) * 100)}%`;

  return (
    <Section
      title="Display"
      description="The part of your screens the tablet is mapped to. Drag it, or use the arrow keys."
    >
      <AreaStage
        width={desktop.width}
        height={desktop.height}
        originX={desktop.minX}
        originY={desktop.minY}
        unit="px"
        label="Screens"
        stageRef={stage}
      >
        {monitors.map((m, index) => (
          <div
            key={`${String(m.x)}:${String(m.y)}:${String(index)}`}
            className="absolute border border-(--stage-line-major) bg-white/5"
            style={{
              left: percent(m.x - desktop.minX, desktop.width),
              top: percent(m.y - desktop.minY, desktop.height),
              width: percent(m.width, desktop.width),
              height: percent(m.height, desktop.height),
            }}
          >
            <span className="absolute right-1.5 bottom-1 text-xs text-(--stage-ink)/75 tabular-nums">
              {m.width}×{m.height}
            </span>
          </div>
        ))}
        <div
          role="group"
          tabIndex={0}
          aria-label="Target area. Arrow keys move it, Shift makes the step larger."
          className="absolute cursor-move touch-none border-[1.5px] border-white bg-(--stage-area) outline-focus-ring focus-visible:outline-2 focus-visible:outline-offset-2"
          style={{
            left: percent(target.x - desktop.minX, desktop.width),
            top: percent(target.y - desktop.minY, desktop.height),
            width: percent(target.w, desktop.width),
            height: percent(target.h, desktop.height),
          }}
          onPointerDown={onPointerDown}
          onPointerMove={onPointerMove}
          onPointerUp={onPointerUp}
          onPointerCancel={onPointerUp}
          onKeyDown={onKeyDown}
        >
          <span className="absolute top-1.5 left-2 text-xs font-medium text-white tabular-nums">
            {target.w} × {target.h} px
          </span>
          <span className="absolute bottom-1.5 left-2 text-xs text-(--stage-ink)/80 tabular-nums">
            ratio {target.h === 0 ? "0" : (target.w / target.h).toFixed(4)}
          </span>
        </div>
      </AreaStage>

      <Row
        label="Snap to the edges of the screens"
        hint="The area sticks to a screen edge when it is near."
      >
        <Toggle
          aria-label="Snap to the edges of the screens"
          isSelected={snapping}
          onChange={(display_snapping) => {
            mapping.update((c) => ({ ...c, display_snapping }));
          }}
        />
      </Row>

      <div className="grid grid-cols-2 gap-4 pt-4 md:grid-cols-4">
        <InputNumber
          label="Width (px)"
          minValue={1}
          formatOptions={{ useGrouping: false }}
          value={target.w}
          onChange={(w) => {
            if (Number.isFinite(w)) {
              setTarget(resizeTarget(target, { w }, desktop));
            }
          }}
        />
        <InputNumber
          label="Height (px)"
          minValue={1}
          formatOptions={{ useGrouping: false }}
          value={target.h}
          onChange={(h) => {
            if (Number.isFinite(h)) {
              setTarget(resizeTarget(target, { h }, desktop));
            }
          }}
        />
        <InputNumber
          label="X (px)"
          formatOptions={{ useGrouping: false }}
          value={centre.x}
          onChange={(x) => {
            if (Number.isFinite(x)) {
              setTarget(centreTarget(target, { x }, desktop));
            }
          }}
        />
        <InputNumber
          label="Y (px)"
          formatOptions={{ useGrouping: false }}
          value={centre.y}
          onChange={(y) => {
            if (Number.isFinite(y)) {
              setTarget(centreTarget(target, { y }, desktop));
            }
          }}
        />
      </div>
    </Section>
  );
}
