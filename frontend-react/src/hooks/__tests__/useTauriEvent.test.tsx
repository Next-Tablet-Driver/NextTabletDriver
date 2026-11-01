import { renderHook } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { useTauriEvent } from "@/hooks/useTauriEvent";

describe("useTauriEvent", () => {
  it("forwards events to the latest handler without re-subscribing", async () => {
    let emit: (value: number) => void = () => undefined;
    const subscribe = vi.fn((callback: (value: number) => void) => {
      emit = callback;
      return Promise.resolve(vi.fn());
    });
    const first = vi.fn();
    const second = vi.fn();
    const { rerender } = renderHook(
      ({ handler }) => {
        useTauriEvent(subscribe, handler);
      },
      { initialProps: { handler: first } },
    );
    await Promise.resolve();

    rerender({ handler: second });
    emit(7);

    expect(subscribe).toHaveBeenCalledTimes(1);
    expect(first).not.toHaveBeenCalled();
    expect(second).toHaveBeenCalledWith(7);
  });

  it("releases the subscription on unmount", async () => {
    const unlisten = vi.fn();
    const subscribe = vi.fn(() => Promise.resolve(unlisten));
    const { unmount } = renderHook(() => {
      useTauriEvent(subscribe, vi.fn());
    });
    await Promise.resolve();

    unmount();

    expect(unlisten).toHaveBeenCalledTimes(1);
  });

  it("releases a subscription that resolves after unmount", async () => {
    const unlisten = vi.fn();
    const subscribe = vi.fn(() => Promise.resolve(unlisten));
    const { unmount } = renderHook(() => {
      useTauriEvent(subscribe, vi.fn());
    });

    unmount();
    await Promise.resolve();
    await Promise.resolve();

    expect(unlisten).toHaveBeenCalledTimes(1);
  });

  it("copes with a subscription that returns nothing (outside Tauri)", async () => {
    const subscribe = vi.fn(() => Promise.resolve(undefined));
    const { unmount } = renderHook(() => {
      useTauriEvent(subscribe, vi.fn());
    });
    await Promise.resolve();
    expect(() => {
      unmount();
    }).not.toThrow();
  });
});
