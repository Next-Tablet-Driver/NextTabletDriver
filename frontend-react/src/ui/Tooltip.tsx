import { useEffect, useRef, useState, type ReactNode } from "react";
import styles from "@/ui/Tooltip.module.css";

/** Shows `text` near the pointer after it has rested on the children for `delay` ms. */
export function Tooltip({
  text = "",
  delay = 1000,
  disabled = false,
  children,
}: {
  text?: string;
  delay?: number;
  disabled?: boolean;
  children?: ReactNode;
}) {
  const [visible, setVisible] = useState(false);
  const [position, setPosition] = useState({ x: 0, y: 0 });
  const timer = useRef<number | undefined>(undefined);

  useEffect(
    () => () => {
      window.clearTimeout(timer.current);
    },
    [],
  );

  return (
    <div
      className={styles["tooltip-wrapper"]}
      onMouseEnter={(event) => {
        if (text === "" || disabled) {
          return;
        }
        setPosition({ x: event.clientX, y: event.clientY });
        timer.current = window.setTimeout(() => {
          setVisible(true);
        }, delay);
      }}
      onMouseLeave={() => {
        window.clearTimeout(timer.current);
        setVisible(false);
      }}
      onMouseMove={(event) => {
        if (!visible && timer.current !== undefined) {
          setPosition({ x: event.clientX, y: event.clientY });
        }
      }}
    >
      {children}
      {visible && !disabled && text !== "" && (
        <div
          className={styles["tooltip-content"]}
          style={{ top: `${String(position.y + 20)}px`, left: `${String(position.x + 10)}px` }}
        >
          {text}
        </div>
      )}
    </div>
  );
}
