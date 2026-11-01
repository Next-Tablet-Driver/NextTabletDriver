import { useEffect, useRef, useState } from "react";

/** The size of an element, kept up to date while it is on screen. */
export function useElementSize<T extends HTMLElement>(): readonly [
  ref: React.RefObject<T | null>,
  size: { width: number; height: number },
] {
  const ref = useRef<T>(null);
  const [size, setSize] = useState({ width: 0, height: 0 });
  useEffect(() => {
    const el = ref.current;
    if (el === null) {
      return undefined;
    }
    const observer = new ResizeObserver(() => {
      setSize({ width: el.clientWidth, height: el.clientHeight });
    });
    observer.observe(el);
    setSize({ width: el.clientWidth, height: el.clientHeight });
    return () => {
      observer.disconnect();
    };
  }, []);
  return [ref, size] as const;
}
