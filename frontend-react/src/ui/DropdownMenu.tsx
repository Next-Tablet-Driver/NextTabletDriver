import { useEffect, useRef, useState, type CSSProperties, type ReactNode } from "react";
import { classes } from "@/ui/classes";
import styles from "@/ui/DropdownMenu.module.css";

type Side = "top" | "bottom" | "left" | "right";
export type Placement = `${Side}-start`;

const MARGIN = 8; // minimum distance from the window edge
const GAP = 8; // space between the anchor and the menu (vertical axis)

const OPPOSITE: Record<Side, Side> = { top: "bottom", bottom: "top", left: "right", right: "left" };

/**
 * A menu placed against its offset parent (the element that opened it): on the side it asked
 * for, or on the opposite side when there is no room, and aligned to the end when it would
 * leave the window.
 */
export function DropdownMenu({
  children,
  placement = "bottom-start",
  className = "",
  style,
}: {
  children?: ReactNode;
  placement?: Placement;
  className?: string;
  style?: CSSProperties;
}) {
  const menu = useRef<HTMLDivElement>(null);
  const [side, setSide] = useState<Side | undefined>(undefined);
  const [alignEnd, setAlignEnd] = useState(false);
  const [ready, setReady] = useState(false);
  const preferred = placement.split("-")[0] as Side;

  useEffect(() => {
    const el = menu.current;
    if (el === null) {
      return undefined;
    }
    const resolve = (): void => {
      const anchor = el.offsetParent;
      if (anchor === null) {
        return;
      }
      const a = anchor.getBoundingClientRect();
      const w = el.offsetWidth;
      const h = el.offsetHeight;
      const vw = window.innerWidth;
      const vh = window.innerHeight;
      const space: Record<Side, number> = {
        top: a.top - MARGIN,
        bottom: vh - a.bottom - MARGIN,
        left: a.left - MARGIN,
        right: vw - a.right - MARGIN,
      };
      const vertical = preferred === "top" || preferred === "bottom";
      const needed = vertical ? h + GAP : w;
      const opposite = OPPOSITE[preferred];
      setSide(
        space[preferred] < needed && space[opposite] > space[preferred] ? opposite : preferred,
      );
      setAlignEnd(
        vertical
          ? a.left + w > vw - MARGIN && a.right - w >= MARGIN
          : a.top + h > vh - MARGIN && a.bottom - h >= MARGIN,
      );
      setReady(true);
    };
    resolve();
    const observer = new ResizeObserver(resolve);
    observer.observe(el);
    window.addEventListener("resize", resolve);
    return () => {
      observer.disconnect();
      window.removeEventListener("resize", resolve);
    };
  }, [preferred]);

  return (
    <div
      ref={menu}
      style={{ "--gap": `${String(GAP)}px`, ...style } as CSSProperties}
      className={classes(styles["dropdown-menu"], className)}
      data-align={alignEnd ? "end" : "start"}
      data-ready={ready ? "true" : "false"}
      data-side={side ?? preferred}
      role="menu"
    >
      {children}
    </div>
  );
}
