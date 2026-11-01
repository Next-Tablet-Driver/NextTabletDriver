import { act, renderHook, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { useCommand } from "@/hooks/useCommand";

describe("useCommand", () => {
  it("goes from loading to ready", async () => {
    const call = vi.fn(() => Promise.resolve("v3.0.0"));
    const { result } = renderHook(() => useCommand(call));

    expect(result.current.state).toEqual({ status: "loading" });
    await waitFor(() => {
      expect(result.current.state).toEqual({ status: "ready", data: "v3.0.0" });
    });
  });

  it("reports the message of a failure", async () => {
    const call = vi.fn(() => Promise.reject(new Error("no tablet")));
    const { result } = renderHook(() => useCommand(call));

    await waitFor(() => {
      expect(result.current.state).toEqual({ status: "error", message: "no tablet" });
    });
  });

  it("reports a backend message that is a plain string", async () => {
    // eslint-disable-next-line @typescript-eslint/prefer-promise-reject-errors -- the backend rejects with text
    const call = vi.fn(() => Promise.reject("permission denied"));
    const { result } = renderHook(() => useCommand(call));

    await waitFor(() => {
      expect(result.current.state).toEqual({ status: "error", message: "permission denied" });
    });
  });

  it("calls again on reload and keeps only the latest answer", async () => {
    const answers = ["first", "second"];
    const call = vi.fn(() => Promise.resolve(answers.shift() ?? "none"));
    const { result } = renderHook(() => useCommand(call));
    await waitFor(() => {
      expect(result.current.state).toEqual({ status: "ready", data: "first" });
    });

    act(() => {
      result.current.reload();
    });

    await waitFor(() => {
      expect(result.current.state).toEqual({ status: "ready", data: "second" });
    });
    expect(call).toHaveBeenCalledTimes(2);
  });

  it("drops the answer that arrives after unmount", async () => {
    let finish: (value: string) => void = () => undefined;
    const call = vi.fn(
      () =>
        new Promise<string>((resolve) => {
          finish = resolve;
        }),
    );
    const { result, unmount } = renderHook(() => useCommand(call));
    unmount();

    finish("late");
    await Promise.resolve();

    expect(result.current.state).toEqual({ status: "loading" });
  });
});
