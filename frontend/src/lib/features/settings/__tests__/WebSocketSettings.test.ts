import { describe, it, expect } from "vitest";
import { render, screen, fireEvent } from "@testing-library/svelte";
import { fakeMapping, type FakeMapping } from "../../../test-support/fakeMapping";
import WebSocketSettings from "../WebSocketSettings.svelte";

function websocketMapping(overrides: Record<string, unknown> = {}, loaded = true): FakeMapping<{ websocket: Record<string, unknown> }> {
    return fakeMapping(
        loaded
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
            : null,
    );
}

describe("WebSocketSettings", () => {
    it("shows the server state, port and rate", () => {
        const { mapping } = websocketMapping();
        render(WebSocketSettings, { props: { mapping } });
        expect(screen.getByText("WebSocket Server")).toBeInTheDocument();
        expect(screen.getByLabelText("Enable WebSocket Server")).not.toBeChecked();
        expect(screen.getByText("STOPPED")).toBeInTheDocument();
        expect(screen.getByDisplayValue("8080")).toBeInTheDocument();
        expect(screen.getByDisplayValue("60")).toBeInTheDocument();
    });

    it("reports a running server", () => {
        const { mapping } = websocketMapping({ enabled: true });
        render(WebSocketSettings, { props: { mapping } });
        expect(screen.getByLabelText("Enable WebSocket Server")).toBeChecked();
        expect(screen.getByText("RUNNING")).toBeInTheDocument();
    });

    it("lets the payload be chosen field by field", () => {
        const { mapping } = websocketMapping();
        render(WebSocketSettings, { props: { mapping } });
        expect(screen.getByLabelText("Coords")).toBeChecked();
        expect(screen.getByLabelText("Pressure")).not.toBeChecked();
        expect(screen.getByLabelText("Tilt")).not.toBeChecked();
        expect(screen.getByLabelText("Status")).toBeChecked();
    });

    it("writes what changes and marks the profile dirty", async () => {
        const { mapping, markDirty, config } = websocketMapping();
        render(WebSocketSettings, { props: { mapping } });
        await fireEvent.click(screen.getByLabelText("Pressure"));
        await fireEvent.input(screen.getByDisplayValue("8080"), { target: { value: "9000" } });
        expect(config?.websocket).toMatchObject({ send_pressure: true, port: 9000, send_tilt: false });
        expect(markDirty).toHaveBeenCalledTimes(2);
    });

    it("shows only the header until the config is loaded", () => {
        const { mapping } = websocketMapping({}, false);
        render(WebSocketSettings, { props: { mapping } });
        expect(screen.getByText("WebSocket Server")).toBeInTheDocument();
        expect(screen.queryAllByRole("checkbox")).toHaveLength(0);
    });
});
