import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/svelte";

const getTabletStatus = vi.fn();
const normalizeBinding = vi.fn();
vi.mock("../../../infrastructure/tauri/commands", () => ({
  getTabletStatus: (...args: unknown[]) => getTabletStatus(...args) as unknown,
  normalizeBinding: (...args: unknown[]) => normalizeBinding(...args) as unknown,
}));

import { fakeMapping } from "../../../test-support/fakeMapping";
import ButtonActions from "../ButtonActions.svelte";

interface Bindings {
  pen_button_bindings: string[];
  aux_button_bindings: string[];
}

const tablet = (pen: number, aux: number): unknown => ({
  connected: true,
  name: "T",
  width: 1,
  height: 1,
  pen_button_count: pen,
  aux_button_count: aux,
});

beforeEach(() => {
  getTabletStatus.mockReset();
  normalizeBinding.mockReset();
  getTabletStatus.mockResolvedValue(tablet(0, 0));
  normalizeBinding.mockImplementation((text: string) => Promise.resolve(text));
});

describe("tablet keys", () => {
  it("shows no section for a tablet without keys", () => {
    const { mapping } = fakeMapping<Bindings>({
      pen_button_bindings: ["None"],
      aux_button_bindings: [],
    });
    render(ButtonActions, { props: { mapping } });
    expect(screen.queryByText("Tablet Keys")).toBeNull();
  });

  it("shows one field per key the tablet says it has", async () => {
    getTabletStatus.mockResolvedValue(tablet(2, 3));
    const { mapping } = fakeMapping<Bindings>({
      pen_button_bindings: ["None", "None"],
      aux_button_bindings: [],
    });
    render(ButtonActions, { props: { mapping } });
    expect(await screen.findByLabelText("Tablet Key 3")).toHaveValue("None");
    expect(screen.getByLabelText("Tablet Key 1")).toBeInTheDocument();
    expect(screen.queryByLabelText("Tablet Key 4")).toBeNull();
  });

  it("shows the saved bindings even before the tablet answers", () => {
    const { mapping } = fakeMapping<Bindings>({
      pen_button_bindings: ["None"],
      aux_button_bindings: ["Ctrl+Z", "Space"],
    });
    render(ButtonActions, { props: { mapping } });
    expect(screen.getByLabelText("Tablet Key 1")).toHaveValue("Ctrl+Z");
    expect(screen.getByLabelText("Tablet Key 2")).toHaveValue("Space");
  });

  it("writes a key binding without touching the pen bindings", async () => {
    const { mapping, markDirty, config } = fakeMapping<Bindings>({
      pen_button_bindings: ["Right Click"],
      aux_button_bindings: ["None"],
    });
    render(ButtonActions, { props: { mapping } });
    await fireEvent.input(screen.getByLabelText("Tablet Key 1"), { target: { value: "Ctrl+S" } });
    expect(config?.aux_button_bindings).toEqual(["Ctrl+S"]);
    expect(config?.pen_button_bindings).toEqual(["Right Click"]);
    expect(markDirty).toHaveBeenCalledOnce();
  });

  it("pads the list with None when a later key is edited first", async () => {
    getTabletStatus.mockResolvedValue(tablet(1, 4));
    const { mapping, config } = fakeMapping<Bindings>({
      pen_button_bindings: ["None"],
      aux_button_bindings: [],
    });
    render(ButtonActions, { props: { mapping } });
    await fireEvent.input(await screen.findByLabelText("Tablet Key 3"), {
      target: { value: "F5" },
    });
    expect(config?.aux_button_bindings).toEqual(["None", "None", "F5"]);
  });
});

describe("pen buttons the tablet has more of than the settings", () => {
  it("adds a field for each missing pen button", async () => {
    getTabletStatus.mockResolvedValue(tablet(3, 0));
    const { mapping } = fakeMapping<Bindings>({
      pen_button_bindings: ["Right Click"],
      aux_button_bindings: [],
    });
    render(ButtonActions, { props: { mapping } });
    expect(await screen.findByLabelText("Pen Button 3")).toHaveValue("None");
  });
});

describe("checking a binding", () => {
  it("stores the plain spelling the driver answers with", async () => {
    normalizeBinding.mockResolvedValue("Ctrl+Z");
    const { mapping, config } = fakeMapping<Bindings>({
      pen_button_bindings: ["None"],
      aux_button_bindings: [],
    });
    render(ButtonActions, { props: { mapping } });
    const field = screen.getByLabelText("Pen Button 1");
    await fireEvent.input(field, { target: { value: "ctrl + z" } });
    await fireEvent.change(field);
    await waitFor(() => {
      expect(config?.pen_button_bindings).toEqual(["Ctrl+Z"]);
    });
    expect(normalizeBinding).toHaveBeenCalledWith("ctrl + z");
  });

  it("shows the driver's message and flags the field when the text is not a binding", async () => {
    normalizeBinding.mockRejectedValue("Unknown key or button: 'Banana'");
    const { mapping } = fakeMapping<Bindings>({
      pen_button_bindings: ["None"],
      aux_button_bindings: [],
    });
    render(ButtonActions, { props: { mapping } });
    const field = screen.getByLabelText("Pen Button 1");
    await fireEvent.input(field, { target: { value: "Banana" } });
    await fireEvent.change(field);
    expect(await screen.findByRole("alert")).toHaveTextContent("Unknown key or button: 'Banana'");
    expect(field).toHaveAttribute("aria-invalid", "true");
  });

  it("clears the message as soon as the text is edited again", async () => {
    normalizeBinding.mockRejectedValue("nope");
    const { mapping } = fakeMapping<Bindings>({
      pen_button_bindings: ["None"],
      aux_button_bindings: [],
    });
    render(ButtonActions, { props: { mapping } });
    const field = screen.getByLabelText("Pen Button 1");
    await fireEvent.change(field);
    await screen.findByRole("alert");
    await fireEvent.input(field, { target: { value: "Space" } });
    expect(screen.queryByRole("alert")).toBeNull();
  });
});

describe("presets", () => {
  it("picking a mouse button sets the binding and the list keeps showing its title", async () => {
    const { mapping, markDirty, config } = fakeMapping<Bindings>({
      pen_button_bindings: ["None"],
      aux_button_bindings: [],
    });
    render(ButtonActions, { props: { mapping } });
    await fireEvent.click(screen.getByRole("button", { name: "Presets" }));
    await fireEvent.click(screen.getByRole("option", { name: "Right Click" }));
    expect(config?.pen_button_bindings).toEqual(["Right Click"]);
    expect(markDirty).toHaveBeenCalled();
    expect(screen.getByRole("button", { name: "Presets" })).toBeInTheDocument();
  });
});

describe("recording a binding", () => {
  function press(code: string, init: KeyboardEventInit = {}): void {
    window.dispatchEvent(
      new KeyboardEvent("keydown", { code, bubbles: true, cancelable: true, ...init }),
    );
  }
  function release(code: string): void {
    window.dispatchEvent(new KeyboardEvent("keyup", { code, bubbles: true, cancelable: true }));
  }

  async function start(): Promise<{ config: Bindings | null }> {
    const { mapping, config } = fakeMapping<Bindings>({
      pen_button_bindings: ["None"],
      aux_button_bindings: [],
    });
    render(ButtonActions, { props: { mapping } });
    await fireEvent.click(screen.getByRole("button", { name: "Record Pen Button 1" }));
    return { config };
  }

  it("binds the key that is pressed", async () => {
    const { config } = await start();
    press("KeyQ");
    expect(config?.pen_button_bindings).toEqual(["Q"]);
  });

  it("binds a combination of modifiers and a key", async () => {
    const { config } = await start();
    press("ControlLeft", { ctrlKey: true });
    press("ShiftLeft", { ctrlKey: true, shiftKey: true });
    press("KeyZ", { ctrlKey: true, shiftKey: true });
    expect(config?.pen_button_bindings).toEqual(["Ctrl+Shift+Z"]);
  });

  it("binds a modifier pressed and released on its own", async () => {
    const { config } = await start();
    press("ShiftLeft", { shiftKey: true });
    release("ShiftLeft");
    expect(config?.pen_button_bindings).toEqual(["Shift"]);
  });

  it("stops listening once a binding is recorded", async () => {
    const { config } = await start();
    press("KeyA");
    press("KeyB");
    expect(config?.pen_button_bindings).toEqual(["A"]);
  });

  it("is cancelled by clicking the recorder again, leaving the binding as it was", async () => {
    const { config } = await start();
    await fireEvent.click(screen.getByRole("button", { name: "Record Pen Button 1" }));
    press("KeyA");
    expect(config?.pen_button_bindings).toEqual(["None"]);
  });

  it("reports a key the driver cannot press and keeps the binding", async () => {
    const { config } = await start();
    press("MediaPlayPause");
    expect(await screen.findByRole("alert")).toHaveTextContent("cannot be used");
    expect(config?.pen_button_bindings).toEqual(["None"]);
  });
});
