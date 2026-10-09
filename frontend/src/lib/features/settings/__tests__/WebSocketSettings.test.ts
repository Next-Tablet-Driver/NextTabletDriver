import { describe, it, expect, vi, type Mock } from "vitest";
import { render, screen, fireEvent } from "@testing-library/svelte";
import type { MappingState } from "../../mapping/mapping-state.svelte";
import WebSocketSettings from "../WebSocketSettings.svelte";

function fakeMapping(overrides: Record<string, unknown> = {}, loaded = true): { mapping: MappingState; markDirty: Mock } {
    const markDirty = vi.fn();
    const config = loaded
        ? {
              websocket: {
                  enabled: false,
                  port: 8080,
                  polling_rate_hz: 60,
                  send_coordinates: true,
                  send_pressure: false,
                  send_tilt: false,
                  send_status: true,
                  ...overrides,
              },
          }
        : null;
    return { mapping: { config, markDirty } as unknown as MappingState, markDirty };
}

describe("WebSocketSettings", () => {
    it("shows the server state, port and rate", () => {
        const { mapping } = fakeMapping();
        render(WebSocketSettings, { props: { mapping } });
        expect(screen.getByText("WebSocket Server")).toBeInTheDocument();
        expect(screen.getByLabelText("Enable WebSocket Server")).not.toBeChecked();
        expect(screen.getByText("STOPPED")).toBeInTheDocument();
        expect(screen.getByDisplayValue("8080")).toBeInTheDocument();
        expect(screen.getByDisplayValue("60")).toBeInTheDocument();
    });

    it("reports a running server", () => {
        const { mapping } = fakeMapping({ enabled: true });
        render(WebSocketSettings, { props: { mapping } });
        expect(screen.getByLabelText("Enable WebSocket Server")).toBeChecked();
        expect(screen.getByText("RUNNING")).toBeInTheDocument();
    });

    it("lets the payload be chosen field by field", () => {
        const { mapping } = fakeMapping();
        render(WebSocketSettings, { props: { mapping } });
        expect(screen.getByLabelText("Coords")).toBeChecked();
        expect(screen.getByLabelText("Pressure")).not.toBeChecked();
        expect(screen.getByLabelText("Tilt")).not.toBeChecked();
        expect(screen.getByLabelText("Status")).toBeChecked();
    });

    it("marks the profile dirty when something changes", async () => {
        const { mapping, markDirty } = fakeMapping();
        render(WebSocketSettings, { props: { mapping } });
        await fireEvent.click(screen.getByLabelText("Pressure"));
        await fireEvent.input(screen.getByDisplayValue("8080"), { target: { value: "9000" } });
        expect(markDirty).toHaveBeenCalledTimes(2);
    });

    it("shows only the header until the config is loaded", () => {
        const { mapping } = fakeMapping({}, false);
        render(WebSocketSettings, { props: { mapping } });
        expect(screen.getByText("WebSocket Server")).toBeInTheDocument();
        expect(screen.queryAllByRole("checkbox")).toHaveLength(0);
    });
});
