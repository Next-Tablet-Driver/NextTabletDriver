import { useEffect, useMemo, useRef, useState, type KeyboardEvent, type PointerEvent } from "react";
import { targetOf, withTarget } from "@/features/mapping/config-areas";
import {
  centreTarget,
  clampTarget,
  desktopOf,
  dragTarget,
  resizeTarget,
  targetCentre,
  type MonitorRect,
  type Rect,
} from "@/features/mapping/geometry";
import { useMappingSnapshot, useMappingStore } from "@/features/mapping/MappingProvider";
import { useElementSize } from "@/hooks/useElementSize";
import { log } from "@/infrastructure/log";
import { windowControls } from "@/infrastructure/tauri/window";
import { Checkbox } from "@/ui/Checkbox";
import { Input } from "@/ui/Input";
import { Title } from "@/ui/Title";
import styles from "@/features/mapping/DisplayVisualizer.module.css";

const KEY_STEP_PX = 10;
const ARROWS: Record<string, [number, number]> = {
  ArrowLeft: [-1, 0],
  ArrowRight: [1, 0],
  ArrowUp: [0, -1],
  ArrowDown: [0, 1],
};

/** The screens of the computer, and the part of them the tablet is mapped to. */
export function DisplayVisualizer() {
  const { config } = useMappingSnapshot();
  const mapping = useMappingStore();
  const [monitors, setMonitors] = useState<MonitorRect[]>([]);
  const [areaRef, area] = useElementSize();
  const canvas = useRef<HTMLDivElement>(null);
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

  const scale =
    area.width > 0 && area.height > 0
      ? Math.min((area.width * 0.95) / desktop.width, (area.height * 0.95) / desktop.height)
      : 0;
  const snapping = config.display_snapping;
  const centre = targetCentre(target, desktop);
  const setTarget = (next: Rect): void => {
    mapping.update((c) => withTarget(c, next));
  };

  const onPointerDown = (e: PointerEvent<HTMLDivElement>): void => {
    drag.current = { x: e.clientX, y: e.clientY, start: target };
    e.currentTarget.setPointerCapture(e.pointerId);
  };
  const onPointerMove = (e: PointerEvent<HTMLDivElement>): void => {
    const started = drag.current;
    const box = canvas.current?.getBoundingClientRect();
    if (started === null || box === undefined || box.width === 0) {
      return;
    }
    const ratio = desktop.width / box.width;
    setTarget(
      dragTarget(
        started.start,
        (e.clientX - started.x) * ratio,
        (e.clientY - started.y) * ratio,
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
    <div className={styles["section-wrapper"]}>
      <Title>Display</Title>
      <div ref={areaRef} className={styles["visual-area"]}>
        <div
          ref={canvas}
          className={styles["display-canvas"]}
          style={{
            width: `${String(desktop.width * scale)}px`,
            height: `${String(desktop.height * scale)}px`,
          }}
        >
          {monitors.map((m, index) => (
            <div
              key={`${String(m.x)}:${String(m.y)}:${String(index)}`}
              className={styles["monitor-bg"]}
              style={{
                left: percent(m.x - desktop.minX, desktop.width),
                top: percent(m.y - desktop.minY, desktop.height),
                height: percent(m.height, desktop.height),
                width: percent(m.width, desktop.width),
              }}
            >
              {m.width}x{m.height}
            </div>
          ))}
          <div
            role="presentation"
            tabIndex={0}
            className={styles["target-area"]}
            style={{
              left: percent(target.x - desktop.minX, desktop.width),
              top: percent(target.y - desktop.minY, desktop.height),
              height: percent(target.h, desktop.height),
              width: percent(target.w, desktop.width),
            }}
            onPointerDown={onPointerDown}
            onPointerMove={onPointerMove}
            onPointerUp={onPointerUp}
            onPointerCancel={onPointerUp}
            onKeyDown={onKeyDown}
          >
            <div className={styles["ratio-text"]}>
              {target.h !== 0 ? (target.w / target.h).toFixed(4) : "0.0000"}
            </div>
            <div className={styles.dimensions}>
              <span className={styles["dim-top"]}>{target.w}px</span>
              <span className={styles["dim-left"]}>{target.h}px</span>
            </div>
          </div>
        </div>
      </div>

      <div className={styles.controls}>
        <div className={styles["checkbox-row"]}>
          <Checkbox
            label="Enable Edge Snapping"
            checked={snapping}
            onChange={(display_snapping) => {
              mapping.update((c) => ({ ...c, display_snapping }));
            }}
          />
        </div>
        <div className={styles["input-row"]}>
          <Input
            label="Width"
            type="number"
            unit="px"
            value={target.w}
            onCommit={(w) => {
              setTarget(resizeTarget(target, { w }, desktop));
            }}
          />
          <Input
            label="Height"
            type="number"
            unit="px"
            value={target.h}
            onCommit={(h) => {
              setTarget(resizeTarget(target, { h }, desktop));
            }}
          />
          <Input
            label="X"
            type="number"
            unit="px"
            value={centre.x}
            onCommit={(x) => {
              setTarget(centreTarget(target, { x }, desktop));
            }}
          />
          <Input
            label="Y"
            type="number"
            unit="px"
            value={centre.y}
            onCommit={(y) => {
              setTarget(centreTarget(target, { y }, desktop));
            }}
          />
        </div>
      </div>
    </div>
  );
}
