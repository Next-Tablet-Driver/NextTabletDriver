import {
  useCallback,
  useEffect,
  useRef,
  useState,
  type KeyboardEvent,
  type PointerEvent,
} from "react";
import { Toggle } from "@/components/base/toggle/toggle";
import { InputNumber } from "@/components/base/input/input-number";
import { AreaStage } from "@/features/mapping/AreaStage";
import { activeOf, targetOf, withActive } from "@/features/mapping/config-areas";
import {
  clampActive,
  clampRotation,
  dragActive,
  moveActive,
  osuPlayfield,
  resizeActive,
  type ActiveArea,
  type Surface,
} from "@/features/mapping/geometry";
import { useMappingSnapshot, useMappingStore } from "@/features/mapping/MappingProvider";
import { useTauriEvent } from "@/hooks/useTauriEvent";
import { log } from "@/infrastructure/log";
import { getTabletStatus, type TabletStatus } from "@/infrastructure/tauri/commands";
import { events, type TabletEventData } from "@/infrastructure/tauri/events";
import { Row, Section } from "@/ui/Page";

const HIDE_PEN_AFTER_MS = 500;
const KEY_STEP_MM = 0.5;
const ARROWS: Record<string, [number, number]> = {
  ArrowLeft: [-1, 0],
  ArrowRight: [1, 0],
  ArrowUp: [0, -1],
  ArrowDown: [0, 1],
};
const NO_SURFACE: Surface & { maxX: number; maxY: number } = {
  width: 0,
  height: 0,
  maxX: 0,
  maxY: 0,
};

const onTabletEvent = (callback: (data: TabletEventData) => void) => events.onTabletEvent(callback);
const onDeviceChanged = (callback: (data: undefined) => void) =>
  events.onDeviceChanged(() => {
    callback(undefined);
  });

function isPenNear(status: string): boolean {
  const lowered = status.toLowerCase();
  return ["hover", "contact", "pen", "active"].some((word) => lowered.includes(word));
}

/** Where the pen is on the tablet, for as long as it is near the surface. */
function usePen(): { x: number; y: number } | null {
  const [pen, setPen] = useState<{ x: number; y: number } | null>(null);
  const timer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);

  useEffect(
    () => () => {
      clearTimeout(timer.current);
    },
    [],
  );

  useTauriEvent(onTabletEvent, (data) => {
    clearTimeout(timer.current);
    if (!isPenNear(data.status)) {
      setPen(null);
      return;
    }
    setPen({ x: data.x, y: data.y });
    timer.current = setTimeout(() => {
      setPen(null);
    }, HIDE_PEN_AFTER_MS);
  });
  return pen;
}

/** The size of the tablet's surface in millimetres and its highest raw coordinates. */
function useSurface(): Surface & { maxX: number; maxY: number } {
  const [surface, setSurface] = useState(NO_SURFACE);

  const refresh = useCallback(() => {
    getTabletStatus().then((status: TabletStatus) => {
      const width = status.width ?? 0;
      const height = status.height ?? 0;
      if (width > 0 && height > 0) {
        setSurface((s) => ({
          width,
          height,
          maxX: status.max_x ?? s.maxX,
          maxY: status.max_y ?? s.maxY,
        }));
      }
    }, log.warn);
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);
  useTauriEvent(onDeviceChanged, refresh);
  return surface;
}

/** The part of the tablet that is used, and what it is mapped to. */
export function TabletEditor() {
  const { config } = useMappingSnapshot();
  const mapping = useMappingStore();
  const surface = useSurface();
  const pen = usePen();
  const stage = useRef<HTMLDivElement>(null);
  const drag = useRef<{ x: number; y: number; start: ActiveArea } | null>(null);

  if (config === null) {
    return null;
  }

  const area = activeOf(config);
  const target = targetOf(config);
  const known = surface.width > 0 && surface.height > 0;
  // Until a tablet reports its size, the area is drawn on its own extent.
  const width = known ? surface.width : Math.max(area.x + area.w / 2, 1);
  const height = known ? surface.height : Math.max(area.y + area.h / 2, 1);
  const lock = config.lock_aspect_ratio;

  const setArea = (next: ActiveArea): void => {
    mapping.update((c) => withActive(c, next));
  };

  const onPointerDown = (e: PointerEvent<HTMLDivElement>): void => {
    drag.current = { x: e.clientX, y: e.clientY, start: area };
    e.currentTarget.setPointerCapture(e.pointerId);
  };
  const onPointerMove = (e: PointerEvent<HTMLDivElement>): void => {
    const started = drag.current;
    const box = stage.current?.getBoundingClientRect();
    if (started === null || box === undefined || box.width === 0) {
      return;
    }
    const scale = width / box.width;
    setArea(
      dragActive(
        started.start,
        (e.clientX - started.x) * scale,
        (e.clientY - started.y) * scale,
        surface,
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
    const step = KEY_STEP_MM * (e.shiftKey ? 10 : 1);
    setArea(
      clampActive({ ...area, x: area.x + arrow[0] * step, y: area.y + arrow[1] * step }, surface),
    );
  };

  const percent = (value: number, total: number): string => `${String((value / total) * 100)}%`;
  const playfield = osuPlayfield(target);
  const penX = pen !== null && surface.maxX > 0 ? (pen.x / surface.maxX) * width : 0;
  const penY = pen !== null && surface.maxY > 0 ? (pen.y / surface.maxY) * height : 0;

  return (
    <Section
      title="Tablet"
      description="The part of the tablet that is used. Drag it, or use the arrow keys."
    >
      <AreaStage width={width} height={height} unit="mm" label="Tablet surface" stageRef={stage}>
        <div
          role="group"
          tabIndex={0}
          aria-label="Active area. Arrow keys move it, Shift makes the step larger."
          className="absolute cursor-move touch-none border-[1.5px] border-white bg-(--stage-area) outline-focus-ring focus-visible:outline-2 focus-visible:outline-offset-2"
          style={{
            left: percent(area.x - area.w / 2, width),
            top: percent(area.y - area.h / 2, height),
            width: percent(area.w, width),
            height: percent(area.h, height),
            transform: `rotate(${String(area.rotation)}deg)`,
          }}
          onPointerDown={onPointerDown}
          onPointerMove={onPointerMove}
          onPointerUp={onPointerUp}
          onPointerCancel={onPointerUp}
          onKeyDown={onKeyDown}
        >
          {config.show_osu_playfield && target.w > 0 && target.h > 0 && (
            <div
              aria-hidden="true"
              className="pointer-events-none absolute border border-dashed border-white/70"
              style={{
                left: `${String(playfield.left)}%`,
                top: `${String(playfield.top)}%`,
                width: `${String(playfield.width)}%`,
                height: `${String(playfield.height)}%`,
              }}
            />
          )}
          <span className="absolute top-1.5 left-2 text-xs font-medium text-white tabular-nums">
            {area.w.toFixed(2)} × {area.h.toFixed(2)} mm
          </span>
          <span className="absolute bottom-1.5 left-2 text-xs text-(--stage-ink)/80 tabular-nums">
            ratio {area.h === 0 ? "0" : (area.w / area.h).toFixed(4)}
          </span>
        </div>
        {pen !== null && surface.maxX > 0 && surface.maxY > 0 && (
          <div aria-hidden="true" className="pointer-events-none absolute inset-0">
            <div
              className="absolute inset-y-0 w-px bg-(--stage-pen)/60"
              style={{ left: percent(penX, width) }}
            />
            <div
              className="absolute inset-x-0 h-px bg-(--stage-pen)/60"
              style={{ top: percent(penY, height) }}
            />
            <div
              className="absolute size-2.5 -translate-x-1/2 -translate-y-1/2 rounded-full bg-(--stage-pen)"
              style={{ left: percent(penX, width), top: percent(penY, height) }}
            />
            <span
              className="absolute -translate-y-full pb-1.5 pl-2 text-xs text-(--stage-pen) tabular-nums"
              style={{ left: percent(penX, width), top: percent(penY, height) }}
            >
              {penX.toFixed(1)}, {penY.toFixed(1)}
            </span>
          </div>
        )}
      </AreaStage>

      <Row label="Keep the aspect ratio" hint="Changing the width changes the height with it.">
        <Toggle
          aria-label="Keep the aspect ratio"
          isSelected={lock}
          onChange={(lock_aspect_ratio) => {
            mapping.update((c) => ({ ...c, lock_aspect_ratio }));
          }}
        />
      </Row>
      <Row
        label="Show the osu! playfield"
        hint="A dashed guide of the part of the screen osu! draws on."
      >
        <Toggle
          aria-label="Show the osu! playfield"
          isSelected={config.show_osu_playfield}
          onChange={(show_osu_playfield) => {
            mapping.update((c) => ({ ...c, show_osu_playfield }));
          }}
        />
      </Row>

      <div className="grid grid-cols-2 gap-4 pt-4 md:grid-cols-5">
        <InputNumber
          label="Width (mm)"
          minValue={1}
          formatOptions={{ maximumFractionDigits: 2, useGrouping: false }}
          value={area.w}
          onChange={(w) => {
            if (Number.isFinite(w)) {
              setArea(resizeActive(area, { w }, surface, lock));
            }
          }}
        />
        <InputNumber
          label="Height (mm)"
          minValue={1}
          formatOptions={{ maximumFractionDigits: 2, useGrouping: false }}
          value={area.h}
          onChange={(h) => {
            if (Number.isFinite(h)) {
              setArea(resizeActive(area, { h }, surface, lock));
            }
          }}
        />
        <InputNumber
          label="X (mm)"
          formatOptions={{ maximumFractionDigits: 2, useGrouping: false }}
          value={area.x}
          onChange={(x) => {
            if (Number.isFinite(x)) {
              setArea(moveActive(area, { x }, surface));
            }
          }}
        />
        <InputNumber
          label="Y (mm)"
          formatOptions={{ maximumFractionDigits: 2, useGrouping: false }}
          value={area.y}
          onChange={(y) => {
            if (Number.isFinite(y)) {
              setArea(moveActive(area, { y }, surface));
            }
          }}
        />
        <InputNumber
          label="Rotation (°)"
          minValue={-360}
          maxValue={360}
          formatOptions={{ maximumFractionDigits: 2, useGrouping: false }}
          value={area.rotation}
          onChange={(rotation) => {
            if (Number.isFinite(rotation)) {
              setArea({ ...area, rotation: clampRotation(rotation) });
            }
          }}
        />
      </div>
    </Section>
  );
}
