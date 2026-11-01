import { useRef, type KeyboardEvent, type PointerEvent } from "react";
import { activeOf, targetOf, withActive } from "@/features/mapping/config-areas";
import {
  clampActive,
  clampRotation,
  dragActive,
  moveActive,
  osuPlayfield,
  resizeActive,
  type ActiveArea,
} from "@/features/mapping/geometry";
import { usePen, useSurface } from "@/features/mapping/hooks";
import { useMappingSnapshot, useMappingStore } from "@/features/mapping/MappingProvider";
import { useElementSize } from "@/hooks/useElementSize";
import { Checkbox } from "@/ui/Checkbox";
import { Input } from "@/ui/Input";
import { Title } from "@/ui/Title";
import styles from "@/features/mapping/TabletVisualizer.module.css";

const KEY_STEP_MM = 0.5;
const ARROWS: Record<string, [number, number]> = {
  ArrowLeft: [-1, 0],
  ArrowRight: [1, 0],
  ArrowUp: [0, -1],
  ArrowDown: [0, 1],
};

/** The part of the tablet that is used, and what it is mapped to. */
export function TabletVisualizer() {
  const { config } = useMappingSnapshot();
  const mapping = useMappingStore();
  const surface = useSurface();
  const pen = usePen();
  const [areaRef, area] = useElementSize();
  const canvas = useRef<HTMLDivElement>(null);
  const drag = useRef<{ x: number; y: number; start: ActiveArea } | null>(null);

  if (config === null) {
    return null;
  }

  const active = activeOf(config);
  const target = targetOf(config);
  const known = surface.width > 0 && surface.height > 0;
  // Until a tablet reports its size, the area is drawn on its own extent.
  const width = known ? surface.width : Math.max(active.x + active.w / 2, 1);
  const height = known ? surface.height : Math.max(active.y + active.h / 2, 1);
  const scale =
    area.width > 0 && area.height > 0
      ? Math.min((area.width * 0.95) / width, (area.height * 0.95) / height)
      : 0;
  const lock = config.lock_aspect_ratio;
  const setArea = (next: ActiveArea): void => {
    mapping.update((c) => withActive(c, next));
  };

  const onPointerDown = (e: PointerEvent<HTMLDivElement>): void => {
    drag.current = { x: e.clientX, y: e.clientY, start: active };
    e.currentTarget.setPointerCapture(e.pointerId);
  };
  const onPointerMove = (e: PointerEvent<HTMLDivElement>): void => {
    const started = drag.current;
    const box = canvas.current?.getBoundingClientRect();
    if (started === null || box === undefined || box.width === 0) {
      return;
    }
    const ratio = width / box.width;
    setArea(
      dragActive(
        started.start,
        (e.clientX - started.x) * ratio,
        (e.clientY - started.y) * ratio,
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
      clampActive(
        { ...active, x: active.x + arrow[0] * step, y: active.y + arrow[1] * step },
        surface,
      ),
    );
  };

  const percent = (value: number, total: number): string => `${String((value / total) * 100)}%`;
  const playfield = osuPlayfield(target);

  return (
    <div className={styles["section-wrapper"]}>
      <Title>Tablet</Title>
      <div ref={areaRef} className={styles["visual-area"]}>
        <div
          ref={canvas}
          className={styles["tablet-canvas"]}
          style={{
            width: `${String(width * scale)}px`,
            height: `${String(height * scale)}px`,
          }}
        >
          <div
            role="presentation"
            tabIndex={0}
            className={styles["active-area"]}
            style={{
              left: percent(active.x - active.w / 2, width),
              top: percent(active.y - active.h / 2, height),
              height: percent(active.h, height),
              width: percent(active.w, width),
              transform: `rotate(${String(active.rotation)}deg)`,
            }}
            onPointerDown={onPointerDown}
            onPointerMove={onPointerMove}
            onPointerUp={onPointerUp}
            onPointerCancel={onPointerUp}
            onKeyDown={onKeyDown}
          >
            {config.show_osu_playfield && target.w > 0 && target.h > 0 && (
              <div
                className={styles["osu-playfield"]}
                style={{
                  width: `${String(playfield.width)}%`,
                  height: `${String(playfield.height)}%`,
                  left: `${String(playfield.left)}%`,
                  top: `${String(playfield.top)}%`,
                }}
              />
            )}
            <div className={styles["ratio-text"]}>
              {active.h !== 0 ? (active.w / active.h).toFixed(4) : "0.0000"}
            </div>
            <div className={styles.dimensions}>
              <span className={styles["dim-top"]}>{active.w.toFixed(2)}mm</span>
              <span className={styles["dim-left"]}>{active.h.toFixed(2)}mm</span>
            </div>
          </div>
          {pen !== null && surface.maxX > 0 && surface.maxY > 0 && (
            <div
              className={styles["pen-cursor"]}
              style={{
                left: percent(pen.x, surface.maxX),
                top: percent(pen.y, surface.maxY),
              }}
            />
          )}
        </div>
      </div>

      <div className={styles.controls}>
        <div
          className={styles["checkbox-row"]}
          style={{
            display: "flex",
            gap: "var(--spacing-md)",
            justifyContent: "space-between",
            alignItems: "center",
          }}
        >
          <div style={{ display: "flex", gap: "var(--spacing-md)" }}>
            <Checkbox
              label="Force Aspect Ratio"
              checked={lock}
              onChange={(lock_aspect_ratio) => {
                mapping.update((c) => ({ ...c, lock_aspect_ratio }));
              }}
            />
            <Checkbox
              label="Show Osu!Playfield"
              checked={config.show_osu_playfield}
              onChange={(show_osu_playfield) => {
                mapping.update((c) => ({ ...c, show_osu_playfield }));
              }}
            />
          </div>
        </div>
        <div className={styles["input-row"]}>
          <Input
            label="Width"
            type="number"
            unit="mm"
            step="0.01"
            precision={2}
            width="65px"
            value={active.w}
            onCommit={(w) => {
              setArea(resizeActive(active, { w }, surface, lock));
            }}
          />
          <Input
            label="Height"
            type="number"
            unit="mm"
            step="0.01"
            precision={2}
            width="65px"
            value={active.h}
            onCommit={(h) => {
              setArea(resizeActive(active, { h }, surface, lock));
            }}
          />
          <Input
            label="X"
            type="number"
            unit="mm"
            precision={2}
            width="65px"
            value={active.x}
            onCommit={(x) => {
              setArea(moveActive(active, { x }, surface));
            }}
          />
          <Input
            label="Y"
            type="number"
            unit="mm"
            precision={2}
            width="65px"
            value={active.y}
            onCommit={(y) => {
              setArea(moveActive(active, { y }, surface));
            }}
          />
          <Input
            label="Rotation"
            type="number"
            unit="°"
            precision={2}
            width="65px"
            value={active.rotation}
            onCommit={(rotation) => {
              setArea({ ...active, rotation: clampRotation(rotation) });
            }}
          />
        </div>
      </div>
    </div>
  );
}
