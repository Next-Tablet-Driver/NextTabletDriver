import { useEffect, useRef, useState, type ChangeEvent } from "react";
import { classes } from "@/ui/classes";
import styles from "@/ui/Input.module.css";

const DRAG_THRESHOLD = 6; // px before a swipe on the input is recognised

interface Props {
  value: string | number;
  /** A number is committed (here as a number) on blur, Enter, drag, and for text as typed. */
  onCommit?: (value: number) => void;
  /** A text field reports each edit as text. */
  onText?: (value: string) => void;
  label?: string;
  unit?: string;
  type?: "text" | "number";
  step?: string | number;
  min?: string | number;
  max?: string | number;
  width?: string;
  precision?: number;
  /** Commit every edit as it is typed, instead of when the field is left. */
  live?: boolean;
  "aria-label"?: string;
}

function format(value: string | number, precision: number | undefined): string {
  if (precision !== undefined && value !== "") {
    return Number(value).toFixed(precision);
  }
  return String(value);
}

/**
 * The number field of the interface: type a value and leave the field to commit it, or drag the
 * label (or swipe the field with a finger) to change it live, hold Shift to go ten times
 * faster, or use the mouse wheel while it has focus.
 */
export function Input({
  value,
  onCommit,
  onText,
  label = "",
  unit = "",
  type = "number",
  step = "1",
  min,
  max,
  width = "50px",
  precision,
  live = false,
  "aria-label": ariaLabel,
}: Props) {
  const inputRef = useRef<HTMLInputElement>(null);
  const labelRef = useRef<HTMLSpanElement>(null);
  const [draft, setDraft] = useState<string | null>(null);
  const latest = useRef({ value, onCommit, step, min, max, precision, type, draft });
  useEffect(() => {
    latest.current = { value, onCommit, step, min, max, precision, type, draft };
  });

  const shown = draft ?? format(value, precision);

  const stepSize = (): number => {
    const s = parseFloat(String(latest.current.step));
    return Number.isNaN(s) ? 1 : s;
  };
  const currentNumber = (): number => {
    const { draft: text, value: current } = latest.current;
    const n = parseFloat(text ?? String(current));
    return Number.isNaN(n) ? 0 : n;
  };
  const clamp = (raw: number): number => {
    const { precision: p, min: lo, max: hi } = latest.current;
    let next =
      p === undefined ? Math.round(raw * 100000) / 100000 : Math.round(raw * 10 ** p) / 10 ** p;
    if (lo !== undefined && lo !== "" && next < Number(lo)) {
      next = Number(lo);
    }
    if (hi !== undefined && hi !== "" && next > Number(hi)) {
      next = Number(hi);
    }
    return next;
  };

  // The mouse wheel, with the field focused. Not passive, or preventDefault would be ignored.
  useEffect(() => {
    const el = inputRef.current;
    if (el === null) {
      return undefined;
    }
    const onWheel = (e: WheelEvent): void => {
      if (latest.current.type !== "number" || document.activeElement !== el) {
        return;
      }
      // The dominant axis: a sideways swipe on a trackpad gives deltaX, not deltaY.
      const delta = Math.abs(e.deltaX) > Math.abs(e.deltaY) ? e.deltaX : e.deltaY;
      if (delta === 0) {
        return;
      }
      e.preventDefault();
      setDraft(String(clamp(currentNumber() + (delta < 0 ? 1 : -1) * stepSize())));
    };
    el.addEventListener("wheel", onWheel, { passive: false });
    return () => {
      el.removeEventListener("wheel", onWheel);
    };
  }, []);

  // Drag. On the label: at once (mouse, touch, pen). On the field: a horizontal touch swipe only,
  // after a threshold, so that selecting text with the mouse is not disturbed.
  useEffect(() => {
    const attach = (el: HTMLElement, fromLabel: boolean): (() => void) => {
      let pending = false;
      let dragging = false;
      let startX = 0;
      let startY = 0;
      let startValue = 0;
      const beginDrag = (e: PointerEvent): void => {
        dragging = true;
        el.setPointerCapture(e.pointerId);
        if (!fromLabel) {
          inputRef.current?.blur();
        }
      };
      const onDown = (e: PointerEvent): void => {
        if (latest.current.type !== "number") {
          return;
        }
        if (e.pointerType === "mouse" && (!fromLabel || e.button !== 0)) {
          return;
        }
        pending = true;
        dragging = false;
        startX = e.clientX;
        startY = e.clientY;
        startValue = currentNumber();
        if (fromLabel) {
          beginDrag(e);
          e.preventDefault(); // no text selection
        }
      };
      const onMove = (e: PointerEvent): void => {
        if (!pending) {
          return;
        }
        const dx = e.clientX - startX;
        if (!dragging) {
          const dy = e.clientY - startY;
          if (Math.abs(dx) < DRAG_THRESHOLD || Math.abs(dx) < Math.abs(dy)) {
            return;
          }
          beginDrag(e);
        }
        const multiplier = e.shiftKey ? 10 : 1;
        const next = clamp(startValue + dx * stepSize() * multiplier);
        setDraft(null);
        latest.current.onCommit?.(next);
      };
      const onUp = (e: PointerEvent): void => {
        if (!pending) {
          return;
        }
        pending = false;
        if (dragging) {
          dragging = false;
          try {
            el.releasePointerCapture(e.pointerId);
          } catch {
            /* already released */
          }
        }
      };
      el.addEventListener("pointerdown", onDown);
      el.addEventListener("pointermove", onMove);
      el.addEventListener("pointerup", onUp);
      el.addEventListener("pointercancel", onUp);
      return () => {
        el.removeEventListener("pointerdown", onDown);
        el.removeEventListener("pointermove", onMove);
        el.removeEventListener("pointerup", onUp);
        el.removeEventListener("pointercancel", onUp);
      };
    };
    const cleanups: (() => void)[] = [];
    if (labelRef.current !== null) {
      cleanups.push(attach(labelRef.current, true));
    }
    if (inputRef.current !== null) {
      cleanups.push(attach(inputRef.current, false));
    }
    return () => {
      for (const cleanup of cleanups) {
        cleanup();
      }
    };
  }, []);

  const commit = (): void => {
    const text = latest.current.draft;
    if (text === null) {
      return;
    }
    setDraft(null);
    if (latest.current.type === "number") {
      const parsed = parseFloat(text);
      if (!Number.isNaN(parsed)) {
        onCommit?.(parsed);
      }
    }
  };

  return (
    <div className={styles["measure-input-group"]}>
      {label !== "" && (
        <span
          ref={labelRef}
          className={classes(styles.label, type === "number" && styles.draggable)}
        >
          {label}
        </span>
      )}
      <div className={styles["input-wrapper"]}>
        <input
          ref={inputRef}
          aria-label={ariaLabel ?? (label === "" ? undefined : label)}
          style={{ width }}
          className={classes(type === "number" && styles["is-number"])}
          max={max}
          min={min}
          step={step}
          type={type}
          value={shown}
          onFocus={() => {
            setDraft(format(value, precision));
          }}
          onChange={(event: ChangeEvent<HTMLInputElement>) => {
            const text = event.currentTarget.value;
            setDraft(text);
            if (type === "text") {
              onText?.(text);
            } else if (live) {
              const parsed = parseFloat(text);
              if (!Number.isNaN(parsed)) {
                onCommit?.(parsed);
              }
            }
          }}
          onBlur={commit}
          onKeyDown={(event) => {
            if (event.key === "Enter") {
              commit();
            }
          }}
        />
        {unit !== "" && <span className={styles.unit}>{unit}</span>}
      </div>
    </div>
  );
}
