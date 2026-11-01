import { useEffect, useState } from "react";

/**
 * The size of an element, kept up to date while it is on screen. Attach the returned callback as
 * the element's `ref`: it works even when the element only appears after the first render.
 */
export function useElementSize(): readonly [
  ref: (element: HTMLElement | null) => void,
  size: { width: number; height: number },
] {
  const [element, setElement] = useState<HTMLElement | null>(null);
  const [size, setSize] = useState({ width: 0, height: 0 });
  useEffect(() => {
    if (element === null) {
      return undefined;
    }
    // Observing reports the initial size too.
    const observer = new ResizeObserver(() => {
      setSize({ width: element.clientWidth, height: element.clientHeight });
    });
    observer.observe(element);
    return () => {
      observer.disconnect();
    };
  }, [element]);
  return [setElement, size] as const;
}
