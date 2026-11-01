import { useCallback, useEffect, useRef, useState } from "react";
import type { Surface } from "@/features/mapping/geometry";
import { useTauriEvent } from "@/hooks/useTauriEvent";
import { log } from "@/infrastructure/log";
import { getTabletStatus, type TabletStatus } from "@/infrastructure/tauri/commands";
import { events, type TabletEventData } from "@/infrastructure/tauri/events";

const HIDE_PEN_AFTER_MS = 500;
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
export function usePen(): { x: number; y: number } | null {
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
export function useSurface(): Surface & { maxX: number; maxY: number } {
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
